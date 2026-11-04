//! Tauri IPC compatibility layer over the standalone SDK.
use crate::launch::LaunchState;
use nous_core::{
    Versioned,
    battles::Battle,
    gacha::Gacha,
    inventory::{Inventory, Relic},
    profile::PlayerProfile,
    snapshots::{Snapshot, Snapshots},
    world::World,
};
use nous_hsr::{Sdk, SnifferStatus};
use std::path::Path;
use tauri::{AppHandle, Manager, State};
#[tauri::command]
pub fn get_profile(sdk: State<'_, Sdk>) -> Versioned<PlayerProfile> {
    sdk.client().query(|a| a.profile.clone())
}
#[tauri::command]
pub fn get_sniffer_status(sdk: State<'_, Sdk>) -> SnifferStatus {
    sdk.status()
}
#[tauri::command]
pub fn set_capture_enabled(
    app: AppHandle,
    sdk: State<'_, Sdk>,
    enabled: bool,
) -> Result<(), String> {
    if crate::launch::is_replay(&app) {
        return sdk.stop_capture();
    }
    if enabled {
        sdk.start_capture()
    } else {
        sdk.stop_capture()
    }
}
#[tauri::command]
pub fn get_snapshot_kinds() -> Vec<(&'static str, &'static str)> {
    nous_core::snapshots::KINDS.to_vec()
}
/// Inventory of the current account (live, or as stored from the last session).
#[tauri::command]
pub fn get_inventory(sdk: State<'_, Sdk>) -> Versioned<Inventory> {
    sdk.client().query(|a| a.inventory.clone())
}

/// Relics of the current account. `uid` is the game's unique_id, stable while
/// the relic exists (levelling, rerolls, equipping), see docs: Данные аккаунта.
#[tauri::command]
pub fn get_relics(sdk: State<'_, Sdk>) -> Versioned<Vec<Relic>> {
    sdk.client()
        .query(|a| a.inventory.relics.values().cloned().collect())
}

/// Location, position, opened chests and object states.
#[tauri::command]
pub fn get_world(sdk: State<'_, Sdk>) -> Versioned<World> {
    sdk.client().query(|a| a.world.clone())
}

/// Recorded battles, newest last.
#[tauri::command]
pub fn get_battles(sdk: State<'_, Sdk>) -> Versioned<Vec<Battle>> {
    sdk.client().query(|a| a.battles.list.clone())
}

/// Banners, warp history and live pulls.
#[tauri::command]
pub fn get_gacha(sdk: State<'_, Sdk>) -> Versioned<Gacha> {
    sdk.client().query(|a| a.gacha.clone())
}

/// Latest raw answers of the game by command name (see snapshots::KINDS).
#[tauri::command]
pub fn get_snapshots(sdk: State<'_, Sdk>) -> Versioned<Snapshots> {
    sdk.client().query(|a| a.snapshots.clone())
}

/// One raw answer, e.g. "GetDailyActiveInfoScRsp".
#[tauri::command]
pub fn get_snapshot(sdk: State<'_, Sdk>, name: String) -> Versioned<Option<Snapshot>> {
    sdk.client().query(|a| a.snapshots.get(&name).cloned())
}

/// Imports the full warp history of the current account through the official
/// API (see the module docs). Returns how many pulls were new.
#[tauri::command]
pub async fn import_gacha_history(app: AppHandle) -> Result<usize, String> {
    let exe = app
        .state::<LaunchState>()
        .game_path()
        .ok_or("путь к игре неизвестен")?;
    let sdk = app.state::<Sdk>();
    let (uid, known) = sdk
        .client()
        .query(|a| (a.uid, a.gacha.history.clone()))
        .data;
    let uid = uid.ok_or("сначала войдите в игру, чтобы Nous узнал аккаунт")?;
    let url = nous_hsr::gacha::find_log_url(Path::new(&exe))?;
    let pulls =
        tauri::async_runtime::spawn_blocking(move || nous_hsr::gacha::download(&url, uid, &known))
            .await
            .map_err(|e| e.to_string())??;

    sdk.client().import_pulls(uid, pulls)
}
