#![allow(dead_code)]
#[path = "../../../../app/src-tauri/src/account.rs"]
mod account;
#[path = "../../../../app/src-tauri/src/battles.rs"]
mod battles;
#[path = "../../../../app/src-tauri/src/gacha.rs"]
mod gacha;
#[path = "../../../../app/src-tauri/src/game.rs"]
mod game;
#[path = "../../../../app/src-tauri/src/inventory.rs"]
mod inventory;
#[path = "../../../../app/src-tauri/src/launch.rs"]
mod launch;
#[path = "../../../../app/src-tauri/src/profile.rs"]
mod profile;
#[path = "../../../../app/src-tauri/src/recorder.rs"]
mod recorder;
#[path = "../../../../app/src-tauri/src/sniffer.rs"]
mod sniffer;
#[path = "../../../../app/src-tauri/src/snapshots.rs"]
mod snapshots;
#[path = "../../../../app/src-tauri/src/veritas.rs"]
mod veritas;
#[path = "../../../../app/src-tauri/src/world.rs"]
mod world;

#[cfg(test)]
mod audit_tests;
