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
use crate::game::{Shared, Versioned};
use crate::snapshots::{Snapshot, Snapshots};
use crate::world::World;
use crate::inventory::{Inventory, InventoryChanges, Relic, SnapshotReport};
use crate::profile::{PlayerProfile, ProfileSource};

/// File layout version. Older layouts are migrated on load (see `migrate`);
/// files that can't be read are set aside, never overwritten.
/// 1: inventory fields at the top level (serde flatten: maps with numeric keys
///    could not be read back). 2: inventory under "inventory".
pub const FORMAT: u32 = 2;
const DIR: &str = "accounts";
const CURRENT_FILE: &str = "current";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub format: u32,
    pub uid: Option<u32>,
    pub profile: PlayerProfile,
    #[serde(default)]
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

#[derive(Clone)]
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
        let path = self.dir.as_ref()?.join(format!("{uid}.json"));
        let text = fs::read_to_string(&path).ok()?;
        let account = serde_json::from_str::<serde_json::Value>(&text)
            .map_err(|e| e.to_string())
            .and_then(migrate)
            .and_then(|v| serde_json::from_value::<Account>(v).map_err(|e| e.to_string()));
        let mut account = match account {
            Ok(a) => a,
            Err(e) => {
                set_aside(&path, &format!("unreadable: {e}"));
                return None;
            }
        };
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
pub fn get_inventory(shared: State<'_, Shared>) -> Versioned<Inventory> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.inventory.clone())
}

/// Relics of the current account. `uid` is the game's unique_id, stable while
/// the relic exists (levelling, rerolls, equipping), see docs: Данные аккаунта.
#[tauri::command]
pub fn get_relics(shared: State<'_, Shared>) -> Versioned<Vec<Relic>> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.inventory.relics.values().cloned().collect())
}

/// Location, position, opened chests and object states.
#[tauri::command]
pub fn get_world(shared: State<'_, Shared>) -> Versioned<World> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.world.clone())
}

/// Recorded battles, newest last.
#[tauri::command]
pub fn get_battles(shared: State<'_, Shared>) -> Versioned<Vec<Battle>> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.battles.list.clone())
}

/// Banners, warp history and live pulls.
#[tauri::command]
pub fn get_gacha(shared: State<'_, Shared>) -> Versioned<Gacha> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.gacha.clone())
}

/// Latest raw answers of the game by command name (see snapshots::KINDS).
#[tauri::command]
pub fn get_snapshots(shared: State<'_, Shared>) -> Versioned<Snapshots> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.snapshots.clone())
}

/// One raw answer, e.g. "GetDailyActiveInfoScRsp".
#[tauri::command]
pub fn get_snapshot(shared: State<'_, Shared>, name: String) -> Versioned<Option<Snapshot>> {
    let game = shared.game.lock().unwrap();
    game.versioned(game.account.snapshots.get(&name).cloned())
}

/// Brings an older file layout up to FORMAT.
fn migrate(mut v: serde_json::Value) -> Result<serde_json::Value, String> {
    let format = v.get("format").and_then(|f| f.as_u64()).unwrap_or(0);
    let obj = v.as_object_mut().ok_or("not an object")?;
    if format == 1 {
        let mut inventory = serde_json::Map::new();
        for key in ["relics", "lightCones", "materials", "avatars", "avatarPaths", "syncedAt"] {
            if let Some(value) = obj.remove(key) {
                inventory.insert(key.into(), value);
            }
        }
        obj.insert("inventory".into(), inventory.into());
        obj.insert("format".into(), 2.into());
    } else if format != u64::from(FORMAT) {
        return Err(format!("format {format} is not {FORMAT}"));
    }
    Ok(v)
}

/// Keeps a file Nous can't use (`<uid>.json.unreadable-<time>`) before a new
/// account replaces it, so nothing is lost: a later version or a person can
/// still recover it.
fn set_aside(path: &Path, why: &str) {
    let kept = path.with_extension(format!("json.unreadable-{}", crate::profile::now()));
    match fs::rename(path, &kept) {
        Ok(()) => log::warn!("{}: {why}; kept as {}, starting over", path.display(), kept.display()),
        Err(e) => log::error!("{}: {why}; could not set it aside ({e})", path.display()),
    }
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
        account.inventory.relics.insert(254, crate::inventory::Relic { uid: 254, tid: 61011, ..Default::default() });
        account.profile.source = ProfileSource::Live;
        store.save(&account);

        let back = store.load_current();
        assert_eq!(back.uid, Some(42));
        assert_eq!(back.profile.nickname.as_deref(), Some("Стелла"));
        assert_eq!(back.profile.source, ProfileSource::Cached, "stored data shows as cached");
        assert_eq!(back.inventory.relics[&254].tid, 61011, "relics keyed by number read back");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn format_1_files_are_migrated() {
        let v1 = serde_json::json!({
            "format": 1, "uid": 5,
            "profile": serde_json::to_value(PlayerProfile::empty()).unwrap(),
            "relics": { "254": { "uid": 254, "tid": 61011, "level": 15 } },
            "syncedAt": 100
        });
        let account: Account = serde_json::from_value(migrate(v1).unwrap()).unwrap();
        assert_eq!(account.format, FORMAT);
        assert_eq!(account.inventory.relics[&254].level, 15);
        assert_eq!(account.inventory.synced_at, Some(100));
    }

    /// Reads a real account file (read-only):
    /// `NOUS_ACCOUNT_FILE=<path> cargo test --lib real_account_file -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_account_file() {
        let text = fs::read_to_string(std::env::var("NOUS_ACCOUNT_FILE").unwrap()).unwrap();
        let v = serde_json::from_str::<serde_json::Value>(&text).unwrap();
        match migrate(v).and_then(|v| serde_json::from_value::<Account>(v).map_err(|e| e.to_string())) {
            Ok(a) => println!("loads: {} relics, {} battles", a.inventory.relics.len(), a.battles.list.len()),
            Err(e) => panic!("does not load: {e}"),
        }
    }

    #[test]
    fn unreadable_file_is_kept_aside_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("nous-account-bad-{}", std::process::id()));
        let accounts = dir.join(DIR);
        fs::create_dir_all(&accounts).unwrap();
        fs::write(accounts.join("7.json"), "{ not json").unwrap();
        let store = Store::new(Some(dir.clone()));

        let fresh = store.load_or_new(7);
        assert!(fresh.inventory.relics.is_empty());
        store.save(&fresh);

        let kept: Vec<_> = fs::read_dir(&accounts)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("7.json.unreadable-"))
            .collect();
        assert_eq!(kept.len(), 1, "the old file survives next to the new one");
        assert_eq!(fs::read_to_string(kept[0].path()).unwrap(), "{ not json");
        fs::remove_dir_all(dir).ok();
    }
}
