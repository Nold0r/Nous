mod ipc;
mod launch;
mod module_config;
mod sniffer;
mod veritas;
use tauri::{Emitter, Manager, RunEvent};
struct Bridge {
    stop: std::sync::mpsc::Sender<()>,
    thread: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
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
            let sdk = nous_hsr::Sdk::new(Some(app.path().app_data_dir()?))
                .map_err(std::io::Error::other)?;
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
            app.manage(sdk);
            launch::setup(app.handle())?;
            // optional, off by default: see veritas.rs
            veritas::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            ipc::import_gacha_history,
            ipc::get_snapshot_kinds,
            veritas::host::get_veritas_overlay,
            veritas::host::get_veritas_battle,
            veritas::host::set_veritas_overlay_settings,
            veritas::host::veritas_overlay_action,
            veritas::host::export_veritas_battle,
            veritas::host::read_veritas_plugin_config,
            veritas::host::write_veritas_plugin_config,
            veritas::host::set_veritas_replay_state,
            veritas::host::inspect_veritas_replay_window,
            veritas::check_veritas_update,
            veritas::get_veritas_status,
            veritas::enable_veritas,
            veritas::disable_veritas,
            ipc::get_sniffer_status,
            ipc::set_capture_enabled,
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
                veritas::host::shutdown(app);
                if let Some(sdk) = app.try_state::<nous_hsr::Sdk>() {
                    if let Err(e) = sdk.stop_capture() {
                        log::error!("SDK shutdown: {e}");
                    }
                }
                if let Some(bridge) = app.try_state::<Bridge>() {
                    bridge.stop.send(()).ok();
                    if let Some(thread) = bridge.thread.lock().unwrap().take() {
                        thread.join().ok();
                    }
                }
            }
        });
}
