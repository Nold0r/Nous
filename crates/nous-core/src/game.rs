//! Serialized domain commands, revisions and account switching, independent of HSR and Tauri.
use crate::account::{Account, AccountStore};
use crate::battles::Battle;
use crate::gacha::{Banner, LivePull, Pull};
use crate::inventory::{Bag, Characters, InventoryChanges, InventorySync};
use crate::profile::PlayerProfile;
use crate::snapshots::Snapshot;
use crate::world::{WorldChange, WorldCommand};
use serde::Serialize;
use std::sync::{Arc, Mutex};
/// A read or an event of the account state, stamped with the state revision.
#[derive(Debug, Clone, Serialize)]
pub struct Versioned<T> {
    pub revision: u64,
    pub data: T,
}

/// What a command changed, so the sniffer knows which events to emit.
#[derive(Debug, Default)]
pub struct Changes {
    /// Revision of the state after this command.
    pub revision: u64,
    /// Another account logged in: everything was replaced.
    pub account_switched: bool,
    pub profile: bool,
    /// The whole inventory was replaced (login, other account).
    pub inventory_reset: bool,
    pub inventory: InventoryChanges,
    pub world: WorldChange,
    /// Floors whose stored data changed (with `WorldChange::Changed`).
    pub world_floors: std::collections::BTreeSet<u32>,
    /// A fight just ended.
    pub battle: Option<Battle>,
    pub gacha: bool,
    /// Name of the raw snapshot that was replaced.
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProfilePatch {
    pub nickname: Option<String>,
    pub level: Option<u32>,
    pub stamina: Option<u32>,
    pub reserve_stamina: Option<u32>,
    pub jade: Option<u64>,
    pub credits: Option<u64>,
    pub avatar: Option<u32>,
}

/// One decoded packet is one command and one revision, even if it updates
/// several domains. Adapters normalize their protocol before calling handle.
#[derive(Debug, Clone, Default)]
pub struct Command {
    pub login: Option<u32>,
    pub profile: Option<ProfilePatch>,
    pub bag: Option<Bag>,
    pub characters: Option<Characters>,
    pub inventory_sync: Option<InventorySync>,
    pub world: Option<WorldCommand>,
    pub battle: Option<Battle>,
    pub banners: Option<Vec<Banner>>,
    pub pulls: Option<Vec<LivePull>>,
    pub snapshot: Option<(String, Snapshot)>,
}

pub struct GameState {
    store: Arc<dyn AccountStore>,
    pub account: Account,
    revision: u64,
    dirty: bool,
    logs: Vec<(u32, serde_json::Value)>,
    // A saver takes this under the state lock, then releases the state before
    // writing. Account switches wait for that write, preventing stale saves.
    pub(crate) writer: Arc<Mutex<()>>,
    /// A login whose account switch failed (the old account couldn't be saved
    /// or the new one read). Until it succeeds, that player's commands are
    /// dropped rather than applied to the previous account; every command
    /// retries the switch.
    pending_login: Option<u32>,
}
impl GameState {
    pub fn new(store: impl AccountStore + 'static) -> Result<Self, String> {
        Self::with_store(Arc::new(store))
    }
    pub fn with_store(store: Arc<dyn AccountStore>) -> Result<Self, String> {
        let account = store.load_current()?;
        Ok(Self {
            store,
            account,
            revision: 0,
            dirty: false,
            logs: Vec::new(),
            writer: Arc::new(Mutex::new(())),
            pending_login: None,
        })
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn versioned<T>(&self, data: T) -> Versioned<T> {
        Versioned {
            revision: self.revision,
            data,
        }
    }
    pub fn profile(&self) -> &PlayerProfile {
        &self.account.profile
    }
    pub fn mark_changed(&mut self) -> u64 {
        self.dirty = true;
        self.revision += 1;
        self.revision
    }
    pub fn take_unsaved(&mut self) -> Option<Account> {
        std::mem::take(&mut self.dirty).then(|| self.account.clone())
    }
    pub(crate) fn pending(&mut self) -> PendingSave {
        PendingSave {
            account: self.take_unsaved(),
            logs: std::mem::take(&mut self.logs),
            store: self.store.clone(),
        }
    }
    pub(crate) fn pending_snapshot(&self) -> PendingSave {
        PendingSave {
            account: self.dirty.then(|| self.account.clone()),
            logs: self.logs.clone(),
            store: self.store.clone(),
        }
    }
    pub(crate) fn acknowledge(&mut self, pending: &PendingSave, revision: u64) {
        if self.revision == revision {
            self.dirty = false;
        }
        if self.logs.starts_with(&pending.logs) {
            self.logs.drain(..pending.logs.len());
        }
    }
    pub(crate) fn restore(&mut self, pending: PendingSave) {
        self.dirty |= pending.account.is_some();
        self.logs.splice(0..0, pending.logs);
    }
    pub fn flush(&mut self) -> Result<(), String> {
        let writer = self.writer.clone();
        let _write = writer.lock().unwrap();
        let pending = self.pending();
        if let Err(e) = pending.write() {
            self.restore(pending);
            return Err(e);
        }
        Ok(())
    }
    pub fn import_pulls(&mut self, uid: u32, pulls: Vec<Pull>, now: i64) -> Result<usize, String> {
        if self.account.uid != Some(uid) {
            return Err("аккаунт сменился во время загрузки".into());
        }
        let added = self.account.gacha.merge(pulls, now);
        self.mark_changed();
        Ok(added)
    }
    /// Time-driven changes, called a few times a second: announces the
    /// position of the last move if it came inside the emit interval.
    pub fn tick(&mut self) -> Option<Changes> {
        self.account.world.take_pending_move()?;
        self.revision += 1;
        Some(Changes {
            revision: self.revision,
            world: WorldChange::Moved,
            ..Changes::default()
        })
    }
    pub fn handle(&mut self, command: &Command) -> Result<Changes, String> {
        let mut ch = Changes::default();
        let now = crate::profile::now();
        if command.login.is_some() {
            self.pending_login = None;
        }
        if let Some(uid) = command.login.or(self.pending_login) {
            if self.account.uid != Some(uid) {
                let switched = self.flush().and_then(|()| self.store.load_or_new(uid));
                match switched {
                    Ok(account) => {
                        self.account = account;
                        self.pending_login = None;
                        ch.account_switched = true;
                        ch.inventory_reset = true;
                    }
                    Err(e) => {
                        self.pending_login = Some(uid);
                        return Err(format!("смена аккаунта на {uid} не удалась, данные этого входа пока не применяются: {e}"));
                    }
                }
            }
        }
        if command.login.is_some() {
            self.account.profile.touch();
            ch.profile = true;
        }
        if let Some(patch) = &command.profile {
            let p = &mut self.account.profile;
            if let Some(v) = &patch.nickname {
                p.nickname = Some(v.clone());
            }
            if let Some(v) = patch.level {
                p.level = Some(v);
            }
            if let Some(v) = patch.stamina {
                p.trailblaze_power = Some(v);
            }
            if let Some(v) = patch.reserve_stamina {
                p.reserved_trailblaze_power = Some(v);
            }
            if let Some(v) = patch.jade {
                p.stellar_jade = Some(v);
            }
            if let Some(v) = patch.credits {
                p.credits = Some(v);
            }
            if let Some(v) = patch.avatar {
                p.avatar_id = Some(v);
            }
            p.touch();
            ch.profile = true;
        }
        if let Some(bag) = &command.bag {
            let report = self.account.inventory.apply_bag(bag, now);
            if let Some(uid) = self.account.uid {
                self.logs.push((
                    uid,
                    serde_json::json!({ "t": now, "event": "login", "report": report }),
                ));
            }
            ch.inventory_reset = true;
        }
        if let Some(data) = &command.characters {
            self.account.inventory.apply_avatars(data);
            ch.inventory.characters = true;
        }
        if let Some(sync) = &command.inventory_sync {
            let changes = self.account.inventory.apply_sync(sync, now);
            if let Some(uid) = self.account.uid {
                for relic in &changes.relics {
                    self.logs.push((
                        uid,
                        serde_json::json!({ "t": now, "event": "relic", "relic": relic }),
                    ));
                }
                if !changes.deleted_relics.is_empty() {
                    self.logs.push((uid, serde_json::json!({ "t": now, "event": "deleted", "uids": changes.deleted_relics })));
                }
            }
            let characters = ch.inventory.characters;
            ch.inventory = changes;
            ch.inventory.characters |= characters;
        }
        if let Some(world) = &command.world {
            ch.world = self.account.world.handle(world);
            ch.world_floors = self.account.world.take_touched();
        }
        if let Some(battle) = &command.battle {
            self.account.battles.push(battle.clone());
            // the stored copy: its numbers were made finite
            ch.battle = self.account.battles.list.last().cloned();
        }
        if let Some(banners) = &command.banners {
            self.account.gacha.banners = banners.clone();
            ch.gacha = true;
        }
        if let Some(pulls) = &command.pulls {
            let live = &mut self.account.gacha.live;
            live.extend(pulls.iter().cloned());
            if live.len() > 500 {
                live.drain(..live.len() - 500);
            }
            ch.gacha = true;
        }
        if let Some((name, snapshot)) = &command.snapshot {
            self.account
                .snapshots
                .insert(name.clone(), snapshot.clone());
            ch.snapshot = Some(name.clone());
        }
        let changed = ch.profile
            || ch.inventory_reset
            || !ch.inventory.is_empty()
            || ch.world == WorldChange::Changed
            || ch.battle.is_some()
            || ch.gacha
            || ch.snapshot.is_some();
        if changed {
            self.mark_changed();
        } else if ch.world == WorldChange::Moved {
            self.revision += 1;
        }
        ch.revision = self.revision;
        Ok(ch)
    }
}
pub(crate) struct PendingSave {
    pub account: Option<Account>,
    logs: Vec<(u32, serde_json::Value)>,
    store: Arc<dyn AccountStore>,
}
impl PendingSave {
    pub fn write(&self) -> Result<(), String> {
        if let Some(account) = &self.account {
            self.store.save(account)?;
        }
        for (uid, line) in &self.logs {
            self.store.append_log(*uid, line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Saves fail while `broken` is set, like a locked file or a full disk.
    #[derive(Default)]
    struct FlakyStore {
        broken: AtomicBool,
        saved: Mutex<Vec<Account>>,
    }
    impl AccountStore for FlakyStore {
        fn load_current(&self) -> Result<Account, String> {
            Ok(Account::empty())
        }
        fn load_or_new(&self, uid: u32) -> Result<Account, String> {
            Ok(self
                .saved
                .lock()
                .unwrap()
                .iter()
                .rev()
                .find(|a| a.uid == Some(uid))
                .cloned()
                .unwrap_or_else(|| Account::new(uid)))
        }
        fn save(&self, account: &Account) -> Result<(), String> {
            if self.broken.load(Ordering::Relaxed) {
                return Err("disk unavailable".into());
            }
            self.saved.lock().unwrap().push(account.clone());
            Ok(())
        }
    }
    fn login(uid: u32) -> Command {
        Command { login: Some(uid), ..Command::default() }
    }
    fn rename(name: &str) -> Command {
        Command {
            profile: Some(ProfilePatch { nickname: Some(name.into()), ..ProfilePatch::default() }),
            ..Command::default()
        }
    }

    #[test]
    fn failed_switch_never_mixes_accounts_and_recovers() {
        let store = Arc::new(FlakyStore::default());
        let mut game = GameState::with_store(store.clone()).unwrap();
        game.handle(&login(1)).unwrap();
        game.handle(&rename("first")).unwrap();

        // account 1 has unsaved changes and the disk is gone: 2 can't log in
        store.broken.store(true, Ordering::Relaxed);
        assert!(game.handle(&login(2)).is_err());
        // player 2's data must not land in account 1
        assert!(game.handle(&rename("second")).is_err());
        assert_eq!(game.account.uid, Some(1));
        assert_eq!(game.account.profile.nickname.as_deref(), Some("first"));

        // the disk is back: the next command completes the switch
        store.broken.store(false, Ordering::Relaxed);
        let ch = game.handle(&rename("second")).unwrap();
        assert!(ch.account_switched);
        assert_eq!(game.account.uid, Some(2));
        assert_eq!(game.account.profile.nickname.as_deref(), Some("second"));
        let first = store.load_or_new(1).unwrap();
        assert_eq!(first.profile.nickname.as_deref(), Some("first"));
    }

    #[test]
    fn logging_back_into_the_current_account_cancels_a_failed_switch() {
        let store = Arc::new(FlakyStore::default());
        let mut game = GameState::with_store(store.clone()).unwrap();
        game.handle(&login(1)).unwrap();
        game.handle(&rename("first")).unwrap();
        store.broken.store(true, Ordering::Relaxed);
        assert!(game.handle(&login(2)).is_err());
        game.handle(&login(1)).unwrap();
        game.handle(&rename("again")).unwrap();
        assert_eq!(game.account.profile.nickname.as_deref(), Some("again"));
    }
}
