mod ipc;
mod launch;
mod module_config;
mod sniffer;
// app/modules/*: entry files, `modules::{setup, shutdown}`, handler_with_modules!
include!(concat!(env!("OUT_DIR"), "/modules.rs"));
use tauri::{Emitter, Manager, RunEvent};
struct Bridge {
    stop: std::sync::mpsc::Sender<()>,
    thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

/// Stops the modules and the capture and writes what isn't on disk yet. Runs
/// at exit and before an update is installed: the Windows updater ends the
/// process without the exit event. Safe to call twice.
pub(crate) fn save_everything(app: &tauri::AppHandle) {
    modules::shutdown(app);
    if let Some(sdk) = app.try_state::<nous_hsr::Sdk>() {
        if let Err(e) = sdk.stop_capture() {
            log::error!("SDK shutdown: {e}");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    sniffer::wait_for_previous();
    tauri::Builder::default()
        // a second launch (e.g. from the Start menu while Nous sits in the tray) brings the window up
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            launch::show_main(app)
        }))
        // self-update from GitHub releases (see lib/updater.ts)
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // links in patch notes and settings open in the browser
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // Unreadable account storage must not keep the application from
            // starting. Without storage nothing is read and nothing is written,
            // so whatever is on disk stays untouched for a later repair.
            let sdk = match nous_hsr::Sdk::new(Some(app.path().app_data_dir()?)) {
                Ok(sdk) => sdk,
                Err(e) => {
                    log::error!("account storage is unavailable, running without it: {e}");
                    nous_hsr::Sdk::new(None).map_err(std::io::Error::other)?
                }
            };
            let events = sdk.subscribe();
            let handle = app.handle().clone();
            let (stop, stopped) = std::sync::mpsc::channel();
            let bridge = std::thread::spawn(move || {
                loop {
                    if stopped.try_recv().is_ok() {
                        break;
                    }
                    match events.recv_timeout(std::time::Duration::from_millis(100)) {
                        Ok(event) => {
                            handle.emit(event.name, event.payload).ok();
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(_) => break,
                    }
                }
            });
            app.manage(Bridge {
                stop,
                thread: std::sync::Mutex::new(Some(bridge)),
            });
            // the account (always shipped) needs the game's login and profile,
            // so capture runs for the whole session, not per module
            if !launch::is_replay(app.handle()) {
                sdk.start_capture().map_err(std::io::Error::other)?;
            }
            app.manage(sdk);
            launch::setup(app.handle())?;
            // every module's Rust side (app/modules/*/module.rs)
            modules::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(handler_with_modules![
            module_config::get_module_configuration,
            module_config::set_module_configuration,
            ipc::get_profile,
            ipc::get_inventory,
            ipc::get_relics,
            ipc::get_world,
            ipc::get_battles,
            ipc::get_gacha,
            ipc::get_snapshots,
            ipc::get_snapshot,
            ipc::get_notes,
            ipc::set_relic_note,
            ipc::set_character_note,
            ipc::import_gacha_history,
            ipc::get_snapshot_kinds,
            ipc::get_sniffer_status,
            ipc::prepare_update,
            sniffer::restart_as_admin,
            launch::get_game_status,
            launch::launch_game,
            launch::get_autostart,
            launch::set_autostart,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // whatever the saver thread hasn't written yet
            if let RunEvent::Exit = event {
                save_everything(app);
                if let Some(bridge) = app.try_state::<Bridge>() {
                    bridge.stop.send(()).ok();
                    if let Some(thread) = bridge.thread.lock().unwrap().take() {
                        thread.join().ok();
                    }
                }
            }
        });
}
