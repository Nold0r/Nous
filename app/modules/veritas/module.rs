//! Optional live damage through Veritas (https://github.com/hessiser/veritas).
//!
//! UNLIKE EVERYTHING ELSE IN NOUS, THIS INJECTS INTO THE GAME: Veritas is a DLL
//! the game loads into its own process; it hooks engine functions and reads
//! battle state from memory. It breaks the game's Terms of Service and may get
//! the account sanctioned. So it is off by default, nothing is put into the
//! game folder until the player turns it on in Settings and accepts the risk,
//! and turning it off removes the file again.
//!
//! Install: the official release asset, checked to be built by
//! `github-actions[bot]` and against its SHA-256, copied next to StarRail.exe
//! as `xluau.dll` (Veritas wiki, method 2; the game loads that name itself).
//! Live data: Veritas serves Socket.IO on 127.0.0.1:1305; Nous connects as a
//! client and forwards every packet as the `veritas` event.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use futures::FutureExt;
use rust_socketio::Payload;
use rust_socketio::asynchronous::ClientBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::launch::LaunchState;
// A module is one entry file; its parts sit next to it in the module folder.
#[path = "geometry.rs"]
mod geometry;
#[path = "host.rs"]
pub mod host;
#[path = "overlay_host.rs"]
pub mod overlay_host;
#[path = "plugin_config.rs"]
mod plugin_config;
#[path = "settings.rs"]
mod settings;
#[path = "window.rs"]
mod window;

/// The app's isolated test mode (see launch.rs); Veritas replays a fake battle in it.
pub use crate::launch::is_replay;

const RELEASE_API: &str = "https://api.github.com/repos/hessiser/veritas/releases/latest";
const TRUSTED_BUILDER: &str = "github-actions[bot]";
const ASSET: &str = "veritas.dll";
/// File name the game loads from its folder.
const INSTALLED_NAME: &str = "xluau.dll";
const SERVER: &str = "http://127.0.0.1:1305";
const SETTINGS_FILE: &str = "veritas.json";
const RETRY: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    enabled: bool,
    /// Unix seconds the player accepted the risk.
    accepted_at: Option<i64>,
    installed: Option<Installed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Installed {
    version: String,
    path: String,
    /// Our copy; any other file with that name is never touched.
    sha256: String,
    /// "4.3.54 - 4.6.52" from the release notes.
    game_range: Option<String>,
}

pub struct VeritasState {
    settings: Mutex<Settings>,
    file: Option<PathBuf>,
    connected: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VeritasStatus {
    pub enabled: bool,
    pub version: Option<String>,
    pub game_range: Option<String>,
    pub game_version: Option<String>,
    /// Game version inside the plugin's range; None if unknown.
    pub compatible: Option<bool>,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VeritasPacket {
    /// OnDamage, OnTurnBegin, OnTurnEnd, OnUseSkill, OnBattleBegin, OnBattleEnd, …
    pub name: String,
    pub data: serde_json::Value,
}

/// Module exit: hide the widgets and keep the battle history.
pub fn shutdown(app: &AppHandle) {
    host::shutdown(app);
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let file = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join(SETTINGS_FILE));
    let settings = file
        .as_ref()
        .and_then(|f| fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    app.manage(VeritasState {
        settings: Mutex::new(settings),
        file,
        connected: AtomicBool::new(false),
    });

    host::setup(app)?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move { client_loop(app).await });
    Ok(())
}

impl VeritasState {
    pub fn enabled(&self) -> bool {
        self.settings.lock().unwrap().enabled
    }
    fn save(&self) {
        let Some(file) = &self.file else { return };
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir).ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(&*self.settings.lock().unwrap()) {
            fs::write(file, json).ok();
        }
    }

    fn status(&self, game_exe: Option<&str>) -> VeritasStatus {
        let s = self.settings.lock().unwrap();
        let game_version = game_exe.and_then(game_version);
        let game_range = s.installed.as_ref().and_then(|i| i.game_range.clone());
        VeritasStatus {
            enabled: s.enabled,
            version: s.installed.as_ref().map(|i| i.version.clone()),
            compatible: match (&game_version, &game_range) {
                (Some(v), Some(r)) => in_range(v, r),
                _ => None,
            },
            game_range,
            game_version,
            connected: self.connected.load(Ordering::Relaxed),
        }
    }
}

/// Keeps a Socket.IO connection to Veritas while it's enabled and the game runs.
async fn client_loop(app: AppHandle) {
    loop {
        tokio::time::sleep(RETRY).await;
        let state = app.state::<VeritasState>();
        let replay = is_replay(&app);
        let running = replay || app.state::<LaunchState>().game_running();
        if (!state.settings.lock().unwrap().enabled && !replay) || !running {
            continue;
        }
        let forward = app.clone();
        let closed = app.clone();
        let client = ClientBuilder::new(SERVER)
            .on_any(move |event, payload, _| {
                let app = forward.clone();
                async move {
                    let data = match payload {
                        Payload::Text(mut values) if values.len() == 1 => values.remove(0),
                        Payload::Text(values) => serde_json::Value::Array(values),
                        _ => serde_json::Value::Null,
                    };
                    host::on_packet(&app, &event.to_string(), data.clone());
                    app.emit(
                        "veritas",
                        VeritasPacket {
                            name: event.to_string(),
                            data,
                        },
                    )
                    .ok();
                }
                .boxed()
            })
            .on("close", move |_, _| {
                let app = closed.clone();
                async move {
                    app.state::<VeritasState>()
                        .connected
                        .store(false, Ordering::Relaxed);
                    host::set_connected(&app, false);
                    app.emit("veritas-status", ()).ok();
                }
                .boxed()
            })
            .connect()
            .await;
        let Ok(client) = client else { continue };
        state.connected.store(true, Ordering::Relaxed);
        host::set_connected(&app, true);
        app.emit("veritas-status", ()).ok();

        // stay until Veritas goes away, the game closes or the option is turned off
        while state.connected.load(Ordering::Relaxed)
            && (replay || app.state::<LaunchState>().game_running())
            && (replay || state.settings.lock().unwrap().enabled)
        {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        client.disconnect().await.ok();
        state.connected.store(false, Ordering::Relaxed);
        host::set_connected(&app, false);
        app.emit("veritas-status", ()).ok();
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    author: Option<Account>,
    assets: Vec<Asset>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

#[derive(Deserialize)]
struct Account {
    login: String,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    uploader: Option<Account>,
    digest: Option<String>,
}

/// Downloads the latest official release and checks it before anything is installed.
fn fetch_release() -> Result<(String, Option<String>, Vec<u8>, String), String> {
    let release = release()?;
    download_release(release)
}
fn download_release(release: Release) -> Result<(String, Option<String>, Vec<u8>, String), String> {
    if release.author.as_ref().map(|a| a.login.as_str()) != Some(TRUSTED_BUILDER) {
        return Err("релиз Veritas собран не GitHub Actions: отказ от установки".into());
    }
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == ASSET)
        .ok_or("в релизе нет veritas.dll")?;
    if asset.uploader.as_ref().map(|a| a.login.as_str()) != Some(TRUSTED_BUILDER) {
        return Err("veritas.dll загружен не GitHub Actions: отказ от установки".into());
    }
    let expected = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .ok_or("у файла нет контрольной суммы")?;
    let mut data = Vec::new();
    use std::io::Read;
    ureq::get(&asset.browser_download_url)
        .call()
        .map_err(|e| format!("загрузка: {e}"))?
        .body_mut()
        .with_config()
        .limit(100 * 1024 * 1024)
        .reader()
        .read_to_end(&mut data)
        .map_err(|e| format!("загрузка: {e}"))?;
    let actual = hex(&Sha256::digest(&data));
    if actual != expected {
        return Err("контрольная сумма veritas.dll не совпала: отказ от установки".into());
    }
    let range = release.body.as_deref().and_then(game_range_from_notes);
    Ok((release.tag_name, range, data, actual))
}

/// "Game Version: 4.3.54 - 4.6.52" in the release notes.
fn game_range_from_notes(body: &str) -> Option<String> {
    let line = body
        .lines()
        .find(|l| l.trim_start().starts_with("Game Version:"))?;
    Some(line.split_once(':')?.1.trim().to_string())
}

/// `game_version=4.6.0` from config.ini next to StarRail.exe.
fn game_version(game_exe: &str) -> Option<String> {
    let ini = fs::read_to_string(Path::new(game_exe).parent()?.join("config.ini")).ok()?;
    ini.lines()
        .find_map(|l| l.strip_prefix("game_version="))
        .map(|v| v.trim().to_string())
}

fn in_range(version: &str, range: &str) -> Option<bool> {
    let parse = |v: &str| {
        v.trim()
            .split('.')
            .map(|x| x.parse::<u32>().ok())
            .collect::<Option<Vec<_>>>()
    };
    let (lo, hi) = range.split_once('-')?;
    let (v, lo, hi) = (parse(version)?, parse(lo)?, parse(hi)?);
    Some(lo <= v && v <= hi)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn file_sha256(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|d| hex(&Sha256::digest(&d)))
}

#[tauri::command]
pub fn get_veritas_status(
    app: AppHandle,
    state: State<'_, VeritasState>,
    launch: State<'_, LaunchState>,
) -> VeritasStatus {
    let path = if is_replay(&app) {
        None
    } else {
        launch.game_path()
    };
    state.status(path.as_deref())
}

/// Installs Veritas into the game folder. `accept_risk` must be true: the
/// player confirmed the injection warning in Settings.
#[tauri::command]
pub async fn enable_veritas(
    app: AppHandle,
    accept_risk: bool,
    update_only: Option<bool>,
) -> Result<VeritasStatus, String> {
    if !accept_risk {
        return Err("нужно подтвердить, что вы принимаете риск".into());
    }
    if is_replay(&app) {
        return Err("В тестовом режиме установка и удаление плагина запрещены".into());
    }
    if update_only.unwrap_or(false) {
        let state = app.state::<VeritasState>();
        let s = state.settings.lock().unwrap();
        if !s.enabled || s.accepted_at.is_none() || s.installed.is_none() {
            return Err("Сначала включите Veritas через предупреждение с принятием риска".into());
        }
    }
    let launch = app.state::<LaunchState>();
    if launch.game_running() {
        return Err("закройте игру: файл кладётся в её папку".into());
    }
    let exe = launch.game_path().ok_or("путь к игре неизвестен")?;
    let target = Path::new(&exe)
        .parent()
        .ok_or("неверный путь к игре")?
        .join(INSTALLED_NAME);
    let state = app.state::<VeritasState>();
    // what is in the game folder now: nothing, our file, the same official
    // release (installed by another Nous profile), or something foreign
    let existing = target.exists().then(|| file_sha256(&target));

    let (version, game_range, data, sha256) = tauri::async_runtime::spawn_blocking(move || {
        fetch_release()
    })
    .await
    .map_err(|e| e.to_string())??;
    if launch.game_running() || launch.game_path().as_deref() != Some(exe.as_str()) {
        return Err(
            "Игра запущена или путь изменился во время загрузки: повторите после закрытия игры"
                .into(),
        );
    }
    let ours = state
        .settings
        .lock()
        .unwrap()
        .installed
        .as_ref()
        .map(|i| i.sha256.clone());
    if let Some(found) = &existing {
        if *found != ours && found.as_deref() != Some(sha256.as_str()) {
            return Err(format!(
                "в папке игры уже есть чужой {INSTALLED_NAME}: Nous его не трогает"
            ));
        }
    }
    if target.exists().then(|| file_sha256(&target)) != existing {
        return Err("Файл в папке игры изменился во время загрузки: установка отменена".into());
    }
    host::prepare_plugin(&app, &exe, &version)?;
    // the very same official file is already there: nothing to write
    if existing.flatten().as_deref() != Some(sha256.as_str()) {
        let tmp = target.with_extension("dll.nous-tmp");
        fs::write(&tmp, &data)
            .and_then(|_| fs::rename(&tmp, &target))
            .map_err(|e| format!("запись в папку игры: {e}"))?;
    }

    {
        let mut s = state.settings.lock().unwrap();
        s.enabled = true;
        s.accepted_at = Some(nous_core::profile::now());
        s.installed = Some(Installed {
            version,
            path: target.display().to_string(),
            sha256,
            game_range,
        });
    }
    state.save();
    log::warn!("Veritas installed into the game folder at the player's request");
    app.emit("veritas-status", ()).ok();
    Ok(state.status(Some(&exe)))
}

/// Removes Veritas from the game folder (only the file Nous put there).
#[tauri::command]
pub fn disable_veritas(app: AppHandle) -> Result<VeritasStatus, String> {
    if is_replay(&app) {
        return Err("В тестовом режиме установка и удаление плагина запрещены".into());
    }
    let launch = app.state::<LaunchState>();
    if launch.game_running() {
        return Err("закройте игру: игра держит файл открытым".into());
    }
    let state = app.state::<VeritasState>();
    let installed = state.settings.lock().unwrap().installed.clone();
    if let Some(i) = installed {
        let path = PathBuf::from(&i.path);
        if path.exists() {
            if file_sha256(&path).as_deref() != Some(i.sha256.as_str()) {
                return Err(format!(
                    "{INSTALLED_NAME} в папке игры изменён не Nous: удалите его вручную, если нужно"
                ));
            }
            fs::remove_file(&path).map_err(|e| format!("удаление: {e}"))?;
        }
    }
    {
        let mut s = state.settings.lock().unwrap();
        s.enabled = false;
        s.installed = None;
    }
    state.save();
    host::set_connected(&app, false);
    app.emit("veritas-status", ()).ok();
    Ok(state.status(launch.game_path().as_deref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Downloads and verifies the real release without installing it:
    /// `cargo test --lib live_release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_release() {
        let (version, range, data, sha) = fetch_release().unwrap();
        println!(
            "Veritas {version}, game {range:?}, {} bytes, sha256 {sha}",
            data.len()
        );
        assert!(data.starts_with(b"MZ"), "a Windows DLL");
    }

    #[test]
    fn reads_the_supported_game_range() {
        let notes =
            "**Full Changelog**: x\r\n\r\nGame Version: 4.3.54 - 4.6.52\r\n\r\n### Changelog";
        let range = game_range_from_notes(notes).unwrap();
        assert_eq!(range, "4.3.54 - 4.6.52");
        assert_eq!(in_range("4.6.0", &range), Some(true));
        assert_eq!(in_range("4.7.0", &range), Some(false));
        assert_eq!(in_range("4.3.0", &range), Some(false));
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
    notes: String,
    game_range: Option<String>,
    prerelease: bool,
}
fn release() -> Result<Release, String> {
    let mut release: Release = ureq::get(RELEASE_API)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    if release.draft || release.author.as_ref().map(|a| a.login.as_str()) != Some(TRUSTED_BUILDER) {
        return Err("Релиз создан не GitHub Actions".into());
    }
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == ASSET)
        .ok_or("В релизе нет veritas.dll")?;
    if asset.uploader.as_ref().map(|a| a.login.as_str()) != Some(TRUSTED_BUILDER)
        || !asset.digest.as_deref().is_some_and(|d| {
            d.strip_prefix("sha256:")
                .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        })
    {
        return Err("Файл не прошёл проверку источника и SHA-256".into());
    }
    release.body.get_or_insert_default();
    Ok(release)
}
#[tauri::command]
pub async fn check_veritas_update() -> Result<UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let release = release()?;
        let notes = release.body.unwrap_or_default();
        let game_range = game_range_from_notes(&notes);
        Ok(UpdateInfo {
            version: release.tag_name,
            notes,
            game_range,
            prerelease: release.prerelease,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
