//! The one storage format for game data: a file per account,
//! `<app data>/accounts/<uid>.json`, holding the profile and the inventory.
//! `accounts/current` names the last account seen, shown at startup until the
//! game connects. Relic changes are also appended to `<uid>.relics.jsonl`, a
//! history used to check how relic ids behave (see docs: Данные аккаунта).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::battles::{Battle, Battles};
use crate::gacha::Gacha;
use crate::game::Shared;
use crate::snapshots::{Snapshot, Snapshots};
use crate::world::World;
use crate::inventory::{Inventory, InventoryChanges, Relic, SnapshotReport};
use crate::profile::{PlayerProfile, ProfileSource};

/// Bumped when the file layout changes incompatibly; older files are dropped.
pub const FORMAT: u32 = 1;
const DIR: &str = "accounts";
const CURRENT_FILE: &str = "current";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub format: u32,
    pub uid: Option<u32>,
    pub profile: PlayerProfile,
    #[serde(flatten)]
    pub inventory: Inventory,
    #[serde(default)]
    pub world: World,
    #[serde(default)]
    pub gacha: Gacha,
    #[serde(default)]
    pub battles: Battles,
    /// Raw answers of the game, see snapshots.rs.
    #[serde(default)]
    pub snapshots: Snapshots,
}

impl Account {
    /// No account linked yet.
    pub fn empty() -> Self {
        Self {
            format: FORMAT,
            uid: None,
            profile: PlayerProfile::empty(),
            inventory: Inventory::default(),
            world: World::default(),
            gacha: Gacha::default(),
            battles: Battles::default(),
            snapshots: Snapshots::default(),
        }
    }

    fn new(uid: u32) -> Self {
        let mut account = Self::empty();
        account.uid = Some(uid);
        account.profile.uid = Some(uid);
        account
    }
}

pub struct Store {
    dir: Option<PathBuf>,
}

impl Store {
    pub fn new(app_data: Option<PathBuf>) -> Self {
        Self { dir: app_data.map(|d| d.join(DIR)) }
    }

    /// The last account seen, marked as cached.
    pub fn load_current(&self) -> Account {
        self.dir
            .as_ref()
            .and_then(|d| fs::read_to_string(d.join(CURRENT_FILE)).ok())
            .and_then(|s| s.trim().parse().ok())
            .and_then(|uid| self.load(uid))
            .unwrap_or_else(Account::empty)
    }

    /// The stored account `uid`, or a new one.
    pub fn load_or_new(&self, uid: u32) -> Account {
        self.load(uid).unwrap_or_else(|| Account::new(uid))
    }

    fn load(&self, uid: u32) -> Option<Account> {
        let text = fs::read_to_string(self.dir.as_ref()?.join(format!("{uid}.json"))).ok()?;
        let mut account: Account = serde_json::from_str(&text)
            .inspect_err(|e| log::warn!("account {uid}: unreadable file, starting over: {e}"))
            .ok()?;
        if account.format != FORMAT {
            log::warn!("account {uid}: format {} is not {FORMAT}, starting over", account.format);
            return None;
        }
        if account.profile.source == ProfileSource::Live {
            account.profile.source = ProfileSource::Cached;
        }
        Some(account)
    }

    pub fn save(&self, account: &Account) {
        let (Some(dir), Some(uid)) = (&self.dir, account.uid) else { return };
        let result = fs::create_dir_all(dir)
            .and_then(|_| serde_json::to_vec(account).map_err(std::io::Error::other))
            .and_then(|json| write_atomic(&dir.join(format!("{uid}.json")), &json))
            .and_then(|_| fs::write(dir.join(CURRENT_FILE), uid.to_string()));
        if let Err(e) = result {
            log::warn!("cannot save account {uid}: {e}");
        }
    }

    /// Login list compared with the stored one.
    pub fn log_snapshot(&self, uid: u32, report: &SnapshotReport, now: i64) {
        self.append_log(uid, &serde_json::json!({ "t": now, "event": "login", "report": report }));
    }

    /// Relics changed or deleted while the game was running.
    pub fn log_changes(&self, uid: u32, changes: &InventoryChanges, now: i64) {
        for relic in &changes.relics {
            self.append_log(uid, &serde_json::json!({ "t": now, "event": "relic", "relic": relic }));
        }
        if !changes.deleted_relics.is_empty() {
            self.append_log(uid, &serde_json::json!({ "t": now, "event": "deleted", "uids": changes.deleted_relics }));
        }
    }

    fn append_log(&self, uid: u32, line: &serde_json::Value) {
        let Some(dir) = &self.dir else { return };
        let file = fs::create_dir_all(dir).and_then(|_| {
            OpenOptions::new().create(true).append(true).open(dir.join(format!("{uid}.relics.jsonl")))
        });
        if let Ok(mut f) = file {
            writeln!(f, "{line}").ok();
        }
    }
}

/// Inventory of the current account (live, or as stored from the last session).
#[tauri::command]
pub fn get_inventory(shared: State<'_, Shared>) -> Inventory {
    shared.game.lock().unwrap().account.inventory.clone()
}

/// Relics of the current account. `uid` is the game's unique_id, stable while
/// the relic exists (levelling, rerolls, equipping), see docs: Данные аккаунта.
#[tauri::command]
pub fn get_relics(shared: State<'_, Shared>) -> Vec<Relic> {
    shared.game.lock().unwrap().account.inventory.relics.values().cloned().collect()
}

/// Location, position, opened chests and object states.
#[tauri::command]
pub fn get_world(shared: State<'_, Shared>) -> World {
    shared.game.lock().unwrap().account.world.clone()
}

/// Recorded battles, newest last.
#[tauri::command]
pub fn get_battles(shared: State<'_, Shared>) -> Vec<Battle> {
    shared.game.lock().unwrap().account.battles.list.clone()
}

/// Banners, warp history and live pulls.
#[tauri::command]
pub fn get_gacha(shared: State<'_, Shared>) -> Gacha {
    shared.game.lock().unwrap().account.gacha.clone()
}

/// Latest raw answers of the game by command name (see snapshots::KINDS).
#[tauri::command]
pub fn get_snapshots(shared: State<'_, Shared>) -> Snapshots {
    shared.game.lock().unwrap().account.snapshots.clone()
}

/// One raw answer, e.g. "GetDailyActiveInfoScRsp".
#[tauri::command]
pub fn get_snapshot(shared: State<'_, Shared>, name: String) -> Option<Snapshot> {
    shared.game.lock().unwrap().account.snapshots.get(&name).cloned()
}

/// Write to a temp file and rename, so a crash never leaves half a file.
fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("nous-account-test-{}", std::process::id()));
        let store = Store::new(Some(dir.clone()));

        let mut account = store.load_or_new(42);
        account.profile.nickname = Some("Стелла".into());
        account.profile.source = ProfileSource::Live;
        store.save(&account);

        let back = store.load_current();
        assert_eq!(back.uid, Some(42));
        assert_eq!(back.profile.nickname.as_deref(), Some("Стелла"));
        assert_eq!(back.profile.source, ProfileSource::Cached, "stored data shows as cached");
        fs::remove_dir_all(dir).ok();
    }
}
