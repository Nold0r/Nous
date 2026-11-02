use super::{
    hotkeys::Hotkeys,
    plugin_config::{self, PluginConfigView, PluginOptions},
    settings::OverlaySettings,
};
use nous_veritas::{Battle, Engine, Snapshot};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    pub target_found: bool,
    pub target_visible: bool,
    pub exclusive_fullscreen: bool,
    pub visible: bool,
    pub message: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub snapshot: Snapshot,
    pub settings: OverlaySettings,
    pub presentation: Presentation,
    pub replay: bool,
    pub hidden: bool,
}
struct Inner {
    engine: Engine,
    settings: OverlaySettings,
    presentation: Presentation,
}
pub struct Host {
    inner: Mutex<Inner>,
    pub root: PathBuf,
    pub replay: bool,
    pub hidden: AtomicBool,
    pub target_available: AtomicBool,
    pub exclusive: AtomicBool,
    pub stop: AtomicBool,
    hotkeys: Hotkeys,
    files: Mutex<()>,
}
impl Host {
    pub fn settings(&self) -> OverlaySettings {
        self.inner.lock().unwrap().settings.clone()
    }
    pub fn snapshot(&self) -> Snapshot {
        self.inner.lock().unwrap().engine.snapshot()
    }
    pub fn state(&self) -> OverlayState {
        let inner = self.inner.lock().unwrap();
        OverlayState {
            snapshot: inner.engine.snapshot(),
            settings: inner.settings.clone(),
            presentation: inner.presentation.clone(),
            replay: self.replay,
            hidden: self.hidden.load(Ordering::Relaxed),
        }
    }
    pub fn update_presentation(&self, app: &AppHandle, presentation: Presentation) {
        let mut inner = self.inner.lock().unwrap();
        if inner.presentation != presentation {
            inner.presentation = presentation;
            drop(inner);
            publish(app);
        }
    }
}
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let replay = super::is_replay(app);
    let root = if replay {
        std::env::var_os("NOUS_VERITAS_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("nous-veritas-replay"))
    } else {
        app.path().app_data_dir()?
    };
    std::fs::create_dir_all(&root).map_err(std::io::Error::other)?;
    let mut engine = Engine::default();
    if let Ok(bytes) = std::fs::read(root.join("veritas-battles.json")) {
        match serde_json::from_slice::<Vec<Battle>>(&bytes) {
            Ok(history) => engine.restore_history(history),
            Err(e) => {
                let backup = root.join(format!(
                    "veritas-battles-unreadable-{}.json",
                    nous_core::profile::now()
                ));
                std::fs::write(&backup, &bytes).map_err(std::io::Error::other)?;
                log::warn!(
                    "Veritas history unreadable ({e}), preserved at {}",
                    backup.display()
                );
            }
        }
    }
    let settings = match std::fs::read(root.join("veritas-overlay.json")) {
        Ok(data) => serde_json::from_slice::<OverlaySettings>(&data)
            .ok()
            .filter(|s| s.validate().is_ok())
            .unwrap_or_default(),
        Err(_) => OverlaySettings::default(),
    };
    app.manage(Host {
        inner: Mutex::new(Inner {
            engine,
            settings,
            presentation: Presentation::default(),
        }),
        root,
        replay,
        hidden: AtomicBool::new(false),
        target_available: AtomicBool::new(true),
        exclusive: AtomicBool::new(false),
        stop: AtomicBool::new(false),
        hotkeys: Hotkeys::start(app.clone()),
        files: Mutex::new(()),
    });
    super::window::setup(app, replay)?;
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut bound = false;
        loop {
            std::thread::sleep(Duration::from_millis(100));
            let host = handle.state::<Host>();
            if host.stop.load(Ordering::Relaxed) {
                break;
            }
            let enabled = host.replay || handle.state::<super::VeritasState>().enabled();
            if enabled != bound {
                let keys = if enabled {
                    host.settings().hotkeys
                } else {
                    BTreeMap::new()
                };
                if let Err(e) = host.hotkeys.configure(&keys) {
                    diagnostic(&handle, e);
                }
                bound = enabled;
            }
            super::window::refresh(&handle);
        }
    });
    Ok(())
}
pub fn publish(app: &AppHandle) {
    if let Some(host) = app.try_state::<Host>() {
        app.emit("veritas-overlay", host.state()).ok();
    }
}
pub fn diagnostic(app: &AppHandle, message: impl Into<String>) {
    let message = message.into();
    log::warn!("Veritas: {message}");
    if let Some(host) = app.try_state::<Host>() {
        let _ = host
            .inner
            .lock()
            .unwrap()
            .engine
            .apply("Error", serde_json::json!({"msg":message}));
        publish(app);
    }
}
fn persist_history(app: &AppHandle) -> Result<(), String> {
    let host = app.state::<Host>();
    let _files = host.files.lock().unwrap();
    let inner = host.inner.lock().unwrap();
    plugin_config::atomic_json(
        &host.root.join("veritas-battles.json"),
        &inner.engine.history(),
    )
}
pub fn on_packet(app: &AppHandle, name: &str, data: serde_json::Value) {
    let host = app.state::<Host>();
    let result = host.inner.lock().unwrap().engine.apply(name, data);
    match result {
        Err(e) => diagnostic(app, e),
        Ok(false) => return,
        Ok(true) => {}
    }
    if name == "OnBattleBegin" {
        host.hidden.store(false, Ordering::Relaxed);
    }
    if name == "OnBattleEnd" {
        if let Err(e) = persist_history(app) {
            diagnostic(app, e);
        }
        let state = host.state();
        if state.settings.auto_export {
            if let Some(battle) = state.snapshot.battle {
                if let Err(e) = export(app, battle.id, "both") {
                    diagnostic(app, e);
                }
            }
        }
    }
    super::window::refresh(app);
    publish(app);
}
pub fn set_connected(app: &AppHandle, connected: bool) {
    let host = app.state::<Host>();
    host.inner.lock().unwrap().engine.set_connected(connected);
    if !connected {
        if let Err(e) = persist_history(app) {
            diagnostic(app, e);
        }
    }
    super::window::refresh(app);
    publish(app);
}
pub fn shutdown(app: &AppHandle) {
    if let Some(host) = app.try_state::<Host>() {
        host.stop.store(true, Ordering::Relaxed);
        host.inner.lock().unwrap().engine.finish_interrupted();
        let _ = persist_history(app);
        if let Some(window) = app.get_webview_window("veritas-overlay") {
            let _ = window.hide();
        }
    }
}
#[tauri::command]
pub fn get_veritas_overlay(host: State<'_, Host>) -> OverlayState {
    host.state()
}
#[tauri::command]
pub fn get_veritas_battle(host: State<'_, Host>, id: u64) -> Option<Battle> {
    host.inner.lock().unwrap().engine.battle(id)
}
#[tauri::command]
pub fn set_veritas_overlay_settings(
    app: AppHandle,
    settings: OverlaySettings,
) -> Result<OverlayState, String> {
    settings.validate()?;
    let host = app.state::<Host>();
    let _files = host.files.lock().unwrap();
    let old = host.settings();
    if old.hotkeys != settings.hotkeys
        && (host.replay || app.state::<super::VeritasState>().enabled())
    {
        host.hotkeys.configure(&settings.hotkeys)?;
    }
    if let Err(error) =
        plugin_config::atomic_json(&host.root.join("veritas-overlay.json"), &settings)
    {
        if old.hotkeys != settings.hotkeys {
            let _ = host.hotkeys.configure(&old.hotkeys);
        }
        return Err(error);
    }
    host.inner.lock().unwrap().settings = settings;
    super::window::refresh(&app);
    publish(&app);
    Ok(host.state())
}
pub fn action(app: &AppHandle, action: &str) -> Result<OverlayState, String> {
    let host = app.state::<Host>();
    match action {
        "toggle" => {
            host.hidden.fetch_xor(true, Ordering::Relaxed);
        }
        "hide" => host.hidden.store(true, Ordering::Relaxed),
        "show" => host.hidden.store(false, Ordering::Relaxed),
        "settings" => {
            crate::launch::show_main(app);
            if let Some(window) = app.get_webview_window("main") {
                window
                    .eval("window.location.hash='/settings'")
                    .map_err(|e| e.to_string())?;
            }
        }
        "reset" | "zoomIn" | "zoomOut" | "zoomReset" => {
            let mut settings = host.settings();
            match action {
                "reset" => settings = OverlaySettings::default(),
                "zoomIn" => settings.scale = (settings.scale + 0.1).min(2.5),
                "zoomOut" => settings.scale = (settings.scale - 0.1).max(0.5),
                _ => settings.scale = 1.0,
            }
            return set_veritas_overlay_settings(app.clone(), settings);
        }
        _ => return Err("Неизвестное действие окна".into()),
    }
    super::window::refresh(app);
    publish(app);
    Ok(host.state())
}
#[tauri::command]
pub fn veritas_overlay_action(app: AppHandle, action: String) -> Result<OverlayState, String> {
    self::action(&app, &action)
}
#[derive(Serialize)]
pub struct Exported {
    directory: String,
    files: Vec<String>,
}
fn export(app: &AppHandle, id: u64, format: &str) -> Result<Exported, String> {
    if !["json", "csv", "both"].contains(&format) {
        return Err("Неизвестный формат экспорта".into());
    }
    let host = app.state::<Host>();
    let _files = host.files.lock().unwrap();
    let battle = host
        .inner
        .lock()
        .unwrap()
        .engine
        .battle(id)
        .ok_or("Бой не найден")?;
    let settings = host.settings();
    let mut dir = settings
        .export_directory
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| host.root.join("exports/veritas"));
    if host.replay {
        safe_replay_path(&host.root, &dir)?;
    }
    if settings.date_folders {
        dir = dir.join(utc_day(battle.started_at));
    }
    dir = dir.join(format!("nous_{}", battle.id));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    if format != "csv" {
        let path = dir.join("veritas.json");
        plugin_config::atomic_json(&path, &battle.export_json(&app.state::<super::VeritasState>().settings.lock().unwrap().installed.as_ref().map(|i|i.version.clone()).unwrap_or_else(|| if host.replay { "0.2.57".into() } else { "unknown".into() })))?;
        files.push(path.display().to_string());
    }
    if format != "json" {
        let path = dir.join("veritas.csv");
        plugin_config::atomic(&path, battle.export_csv().as_bytes())?;
        files.push(path.display().to_string());
    }
    Ok(Exported {
        directory: dir.display().to_string(),
        files,
    })
}
fn utc_day(ms: i64) -> String {
    let z = ms.div_euclid(86_400_000) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    format!(
        "{:04}-{:02}-{:02}",
        y + if month <= 2 { 1 } else { 0 },
        month,
        day
    )
}
#[tauri::command]
pub fn export_veritas_battle(app: AppHandle, id: u64, format: String) -> Result<Exported, String> {
    export(&app, id, &format)
}
fn plugin_locations(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let host = app.state::<Host>();
    if host.replay {
        return Ok((
            host.root.join("plugin"),
            host.root.join("plugin/fake-game/veritas.update.json"),
        ));
    }
    let state = app.state::<super::VeritasState>();
    let settings = state.settings.lock().unwrap();
    let installed = settings
        .installed
        .as_ref()
        .ok_or("Сначала установите официальную сборку Veritas после принятия риска")?;
    let root = app
        .path()
        .local_data_dir()
        .map_err(|e| e.to_string())?
        .join("veritas");
    Ok((
        root,
        PathBuf::from(&installed.path)
            .parent()
            .ok_or("Неизвестна папка плагина")?
            .join("veritas.update.json"),
    ))
}
#[tauri::command]
pub fn read_veritas_plugin_config(app: AppHandle) -> Result<PluginConfigView, String> {
    let (root, update) = plugin_locations(&app)?;
    Ok(PluginConfigView {
        directory: root.display().to_string(),
        options: plugin_config::read(&root, &update)?,
        requires_restart: true,
        warning: plugin_config::WARNING,
    })
}
#[tauri::command]
pub fn write_veritas_plugin_config(
    app: AppHandle,
    options: PluginOptions,
) -> Result<PluginConfigView, String> {
    let host = app.state::<Host>();
    if !host.replay && app.state::<crate::launch::LaunchState>().game_running() {
        return Err(
            "Закройте игру: Veritas читает настройки при запуске и может перезаписать изменения"
                .into(),
        );
    }
    let _files = host.files.lock().unwrap();
    let (root, update) = plugin_locations(&app)?;
    let version = if host.replay {
        "0.2.57".into()
    } else {
        app.state::<super::VeritasState>()
            .settings
            .lock()
            .unwrap()
            .installed
            .as_ref()
            .map(|i| i.version.trim_start_matches('v').to_string())
            .ok_or("Плагин не установлен")?
    };
    plugin_config::write(&root, &update, &version, &options)?;
    read_veritas_plugin_config(app.clone())
}
pub fn prepare_plugin(app: &AppHandle, game_exe: &str, version: &str) -> Result<(), String> {
    let root = app
        .path()
        .local_data_dir()
        .map_err(|e| e.to_string())?
        .join("veritas");
    let update = PathBuf::from(game_exe)
        .parent()
        .ok_or("Неверный путь игры")?
        .join("veritas.update.json");
    let mut options = plugin_config::read(&root, &update)?;
    options.suppress_native = true;
    plugin_config::write(&root, &update, version.trim_start_matches('v'), &options)
}
#[tauri::command]
pub fn set_veritas_replay_state(
    app: AppHandle,
    target_visible: bool,
    exclusive_fullscreen: bool,
) -> Result<OverlayState, String> {
    let host = app.state::<Host>();
    if !host.replay {
        return Err("Команда доступна только в изолированном debug-replay".into());
    }
    host.target_available
        .store(target_visible, Ordering::Relaxed);
    host.exclusive
        .store(exclusive_fullscreen, Ordering::Relaxed);
    super::window::refresh(&app);
    publish(&app);
    Ok(host.state())
}

fn safe_replay_path(root: &std::path::Path, path: &std::path::Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        || !path.starts_with(root)
    {
        return Err("Тестовый экспорт разрешён только внутри NOUS_VERITAS_TEST_ROOT".into());
    }
    let root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let existing = path
        .ancestors()
        .find(|p| p.exists())
        .ok_or("Папка экспорта не найдена")?;
    if !std::fs::canonicalize(existing)
        .map_err(|e| e.to_string())?
        .starts_with(root)
    {
        return Err("Ссылка в пути экспорта выходит за тестовую папку".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_export_rejects_parent_traversal() {
        let root =
            std::env::temp_dir().join(format!("nous-export-boundary-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        assert!(safe_replay_path(&root, &root.join("exports")).is_ok());
        assert!(safe_replay_path(&root, &root.join("../outside")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// Native evidence for the isolated replay harness; never available in normal launches.
#[tauri::command]
pub fn inspect_veritas_replay_window(app:AppHandle)->Result<serde_json::Value,String>{
    if !super::is_replay(&app){return Err("Только debug-replay".into());}
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    let w=app.get_webview_window("veritas-overlay").ok_or("Нет окна")?;
    let hwnd=w.hwnd().map_err(|e|e.to_string())?.0;
    let style=unsafe{GetWindowLongPtrW(hwnd,GWL_EXSTYLE)} as u32;
    let pos=w.outer_position().map_err(|e|e.to_string())?;
    let size=w.inner_size().map_err(|e|e.to_string())?;
    Ok(serde_json::json!({"visible":w.is_visible().unwrap_or(false),"focused":w.is_focused().unwrap_or(false),"clickThrough":style&WS_EX_TRANSPARENT!=0,"noActivate":style&WS_EX_NOACTIVATE!=0,"topmost":style&WS_EX_TOPMOST!=0,"toolWindow":style&WS_EX_TOOLWINDOW!=0,"x":pos.x,"y":pos.y,"width":size.width,"height":size.height}))
}
