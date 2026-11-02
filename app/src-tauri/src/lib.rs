mod game;
mod profile;
mod sniffer;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      app.manage(game::Shared {
        game: Mutex::new(game::GameState::load(app.handle())),
        status: Mutex::new(sniffer::SnifferStatus::Starting),
      });
      sniffer::start(app.handle().clone());
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      profile::get_profile,
      sniffer::get_sniffer_status,
      sniffer::restart_as_admin,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
