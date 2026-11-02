//! Running alongside the game: Nous lives in the tray, can start with Windows
//! in the background (elevated, through Task Scheduler, so no UAC prompt), pops
//! its window up when the game starts, and can launch the game itself.

use std::ffi::OsStr;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

/// Command line flag of the autostart task: start hidden in the tray.
pub const BACKGROUND_ARG: &str = "--background";
const GAME_EXE: &str = "StarRail.exe";
const TASK_NAME: &str = "Nous";
const GAME_PATH_FILE: &str = "game_path.txt";
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Keeps console windows of helper processes (powershell, schtasks) hidden.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Emitted as the `game-status` event.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    pub running: bool,
    /// StarRail.exe, if Nous knows where the game is installed.
    pub path: Option<String>,
}

pub struct LaunchState {
    status: Mutex<GameStatus>,
}

impl LaunchState {
    /// StarRail.exe, if known.
    pub fn game_path(&self) -> Option<String> {
        self.status.lock().unwrap().path.clone()
    }

    pub fn game_running(&self) -> bool {
        self.status.lock().unwrap().running
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    app.manage(LaunchState {
        status: Mutex::new(GameStatus { running: false, path: find_game(app).map(|p| p.display().to_string()) }),
    });

    let open = MenuItem::with_id(app, "open", "Открыть Nous", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("bundle icon"))
        .tooltip("Nous")
        .menu(&Menu::with_items(app, &[&open, &quit])?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;

    // closing the window keeps Nous in the tray: the sniffer has to be running
    // when the player logs in
    if let Some(window) = app.get_webview_window("main") {
        let w = window.clone();
        window.on_window_event(move |e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                w.hide().ok();
            }
        });
        if !std::env::args().any(|a| a == BACKGROUND_ARG) {
            window.show().ok();
        }
    }

    let app = app.clone();
    std::thread::spawn(move || watch_game(app));
    Ok(())
}

/// Shows, restores and focuses the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        w.show().ok();
        w.unminimize().ok();
        w.set_focus().ok();
    }
}

/// Polls the process list; brings Nous up when the game starts and remembers
/// where the game is installed.
fn watch_game(app: AppHandle) {
    let mut sys = System::new();
    let refresh = ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet);
    loop {
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
        let game = sys.processes_by_exact_name(OsStr::new(GAME_EXE)).next();
        let running = game.is_some();
        // the exe path is readable only when Nous is elevated like the game
        let seen_path = game.and_then(|p| p.exe()).map(Path::to_path_buf);

        let state = app.state::<LaunchState>();
        let update = {
            let mut s = state.status.lock().unwrap();
            let started = running && !s.running;
            let mut changed = running != s.running;
            s.running = running;
            if let Some(path) = seen_path {
                let path = path.display().to_string();
                if s.path.as_deref() != Some(&path) {
                    remember_game(&app, &path);
                    s.path = Some(path);
                    changed = true;
                }
            }
            changed.then(|| (s.clone(), started))
        };
        if let Some((status, started)) = update {
            if started {
                show_main(&app);
            }
            app.emit("game-status", &status).ok();
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// StarRail.exe: the last path seen running, else the HoYoPlay install path
/// from the registry.
fn find_game(app: &AppHandle) -> Option<PathBuf> {
    let remembered = app
        .path()
        .app_data_dir()
        .ok()
        .and_then(|d| fs::read_to_string(d.join(GAME_PATH_FILE)).ok())
        .map(|s| PathBuf::from(s.trim()));
    remembered.filter(|p| p.is_file()).or_else(game_from_registry)
}

fn remember_game(app: &AppHandle, path: &str) {
    if let Ok(dir) = app.path().app_data_dir() {
        if fs::create_dir_all(&dir).is_ok() {
            fs::write(dir.join(GAME_PATH_FILE), path).ok();
        }
    }
}

/// HoYoPlay writes `GameInstallPath` under `Software\<publisher>\HYP\<launcher
/// version>\hkrpg_<region>`, sometimes one level deeper (standalone installs).
fn game_from_registry() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    fn search(key: &RegKey, depth: u32) -> Option<PathBuf> {
        for name in key.enum_keys().flatten() {
            let Ok(sub) = key.open_subkey(&name) else { continue };
            if name.starts_with("hkrpg_") {
                if let Ok(dir) = sub.get_value::<String, _>("GameInstallPath") {
                    let exe = Path::new(&dir).join(GAME_EXE);
                    if exe.is_file() {
                        return Some(exe);
                    }
                }
            }
            if depth > 0 {
                if let Some(found) = search(&sub, depth - 1) {
                    return Some(found);
                }
            }
        }
        None
    }

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    ["Software\\Cognosphere\\HYP", "Software\\miHoYo\\HYP"]
        .iter()
        .filter_map(|root| hkcu.open_subkey(root).ok())
        .find_map(|key| search(&key, 3))
}

/// Runs a PowerShell script; through a UAC prompt when Nous isn't elevated.
fn powershell(script: &str, elevate: bool) -> Result<(), String> {
    let mut cmd = Command::new("powershell");
    cmd.creation_flags(CREATE_NO_WINDOW).args(["-NoProfile", "-ExecutionPolicy", "Bypass"]);
    if elevate && !is_elevated::is_elevated() {
        let file = std::env::temp_dir().join("nous-task.ps1");
        fs::write(&file, script).map_err(|e| e.to_string())?;
        cmd.args([
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList \
                 '-NoProfile','-ExecutionPolicy','Bypass','-File','\"{}\"'",
                file.display()
            ),
        ]);
    } else {
        cmd.args(["-Command", script]);
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        // declined UAC prompt and script errors land here
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[tauri::command]
pub fn get_game_status(state: State<'_, LaunchState>) -> GameStatus {
    state.status.lock().unwrap().clone()
}

/// Starts the game. Start-Process goes through the shell, so the game's own
/// admin request works whether Nous is elevated or not.
#[tauri::command]
pub fn launch_game(state: State<'_, LaunchState>) -> Result<(), String> {
    let path = state.status.lock().unwrap().path.clone().ok_or("Путь к игре неизвестен")?;
    let dir = Path::new(&path).parent().ok_or("Неверный путь к игре")?;
    powershell(
        &format!(
            "Start-Process -FilePath {} -WorkingDirectory {}",
            ps_quote(&path),
            ps_quote(&dir.display().to_string())
        ),
        false,
    )
}

/// Whether the autostart task exists.
#[tauri::command]
pub fn get_autostart() -> bool {
    Command::new("schtasks")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/Query", "/TN", TASK_NAME])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Creates or removes the logon task that starts Nous in the tray with the
/// highest privileges. Asks for UAC once if Nous isn't elevated.
#[tauri::command]
pub async fn set_autostart(enabled: bool) -> Result<bool, String> {
    let script = if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        format!(
            "$user = \"$env:USERDOMAIN\\$env:USERNAME\"
$action = New-ScheduledTaskAction -Execute {exe} -Argument '{BACKGROUND_ARG}'
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $user
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0
Register-ScheduledTask -TaskName {task} -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null",
            exe = ps_quote(&exe.display().to_string()),
            task = ps_quote(TASK_NAME),
        )
    } else {
        format!("Unregister-ScheduledTask -TaskName {} -Confirm:$false", ps_quote(TASK_NAME))
    };
    tauri::async_runtime::spawn_blocking(move || powershell(&script, true))
        .await
        .map_err(|e| e.to_string())??;
    Ok(get_autostart())
}
