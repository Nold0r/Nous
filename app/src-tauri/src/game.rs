//! Game state rebuilt from the decrypted game commands: the current account
//! (profile and inventory). Stored in the one account format, see account.rs.
//!
//! Every change bumps `revision`; reads (`Versioned`) and events carry it, so
//! the UI can tell which of a snapshot and an event is newer. Writing to disk
//! happens off the packet path: `handle` only marks the account dirty, a saver
//! thread writes it (see lib.rs), and a switch of account flushes first.

use std::sync::Mutex;

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::account::{Account, Store};
use crate::battles::Battle;
use crate::inventory::InventoryChanges;
use crate::world::WorldChange;
use crate::profile::{PlayerProfile, now};
use crate::sniffer::SnifferStatus;

/// App-wide state managed by Tauri.
pub struct Shared {
    pub game: Mutex<GameState>,
    pub status: Mutex<SnifferStatus>,
}

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
    /// A fight just ended.
    pub battle: Option<Battle>,
    pub gacha: bool,
    /// Name of the raw snapshot that was replaced.
    pub snapshot: Option<String>,
}

pub struct GameState {
    store: Store,
    pub account: Account,
    revision: u64,
    /// Changed since the last write.
    dirty: bool,
}

impl GameState {
    pub fn load(app: &AppHandle) -> Self {
        Self::new(Store::new(app.path().app_data_dir().ok()))
    }

    /// State over a store; the last account seen is loaded from it.
    pub fn new(store: Store) -> Self {
        let account = store.load_current();
        Self { store, account, revision: 0, dirty: false }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Stamps a read with the current revision.
    pub fn versioned<T>(&self, data: T) -> Versioned<T> {
        Versioned { revision: self.revision, data }
    }

    /// Records a change made outside `handle` (e.g. an import); returns the new revision.
    pub fn mark_changed(&mut self) -> u64 {
        self.dirty = true;
        self.revision += 1;
        self.revision
    }

    /// The account to write, if it changed since the last call (saver thread).
    pub fn take_unsaved(&mut self) -> Option<Account> {
        std::mem::take(&mut self.dirty).then(|| self.account.clone())
    }

    pub fn store(&self) -> Store {
        self.store.clone()
    }

    /// Writes now if there are unsaved changes (account switch, exit).
    pub fn flush(&mut self) {
        if std::mem::take(&mut self.dirty) {
            self.store.save(&self.account);
        }
    }

    pub fn profile(&self) -> &PlayerProfile {
        &self.account.profile
    }

    /// Applies one decrypted game command.
    pub fn handle(&mut self, command: &GameCommand) -> Changes {
        let mut ch = Changes::default();
        let now = now();
        match command.command_id {
            cmd::PlayerGetTokenScRsp => {
                if let Ok(rsp) = command.parse_proto::<proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp>() {
                    if self.account.uid != Some(rsp.uid) {
                        // the previous account's last changes go to its own file first
                        self.flush();
                        self.account = self.store.load_or_new(rsp.uid);
                        ch.account_switched = true;
                        ch.inventory_reset = true;
                    }
                    self.account.profile.touch();
                    ch.profile = true;
                }
            }
            cmd::PlayerLoginScRsp => {
                if let Ok(rsp) = command.parse_proto::<proto::PlayerLoginScRsp::PlayerLoginScRsp>() {
                    if let Some(info) = rsp.basic_info.as_ref() {
                        self.apply_basic_info(info);
                    }
                    ch.profile = true;
                }
            }
            cmd::PlayerSyncScNotify => {
                if let Ok(sync) = command.parse_proto::<proto::PlayerSyncScNotify::PlayerSyncScNotify>() {
                    if let Some(info) = sync.basic_info.as_ref() {
                        self.apply_basic_info(info);
                        ch.profile = true;
                    }
                    ch.inventory = self.account.inventory.apply_sync(&sync, now);
                    if let Some(uid) = self.account.uid {
                        self.store.log_changes(uid, &ch.inventory, now);
                    }
                }
            }
            cmd::StaminaInfoScNotify => {
                if let Ok(n) = command.parse_proto::<proto::StaminaInfoScNotify::StaminaInfoScNotify>() {
                    let p = &mut self.account.profile;
                    p.trailblaze_power = Some(n.stamina);
                    p.reserved_trailblaze_power = Some(n.reserve_stamina);
                    p.touch();
                    ch.profile = true;
                }
            }
            cmd::GetPlayerBoardDataScRsp => {
                if let Ok(rsp) = command.parse_proto::<proto::GetPlayerBoardDataScRsp::GetPlayerBoardDataScRsp>() {
                    if rsp.current_head_icon_id != 0 {
                        self.account.profile.avatar_id = Some(rsp.current_head_icon_id);
                        self.account.profile.touch();
                        ch.profile = true;
                    }
                }
            }
            cmd::GetBagScRsp => {
                if let Ok(bag) = command.parse_proto::<proto::GetBagScRsp::GetBagScRsp>() {
                    let report = self.account.inventory.apply_bag(&bag, now);
                    log::info!("bag: {} relics, ids vs stored: {report:?}", self.account.inventory.relics.len());
                    if let Some(uid) = self.account.uid {
                        self.store.log_snapshot(uid, &report, now);
                    }
                    ch.inventory_reset = true;
                }
            }
            cmd::GetAvatarDataScRsp => {
                if let Ok(data) = command.parse_proto::<proto::GetAvatarDataScRsp::GetAvatarDataScRsp>() {
                    self.account.inventory.apply_avatars(&data);
                    ch.inventory.characters = true;
                }
            }
            _ => {}
        }

        let account = &mut self.account;
        ch.world = account.world.handle(command);
        ch.battle = account.battles.handle(command, now);
        ch.gacha = account.gacha.handle(command, now);
        if crate::snapshots::handle(&mut account.snapshots, command, now) {
            ch.snapshot = command.get_command_name().map(str::to_string);
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
            // movement alone isn't worth a write: the position is saved with the next change
            self.revision += 1;
        }
        ch.revision = self.revision;
        ch
    }

    fn apply_basic_info(&mut self, info: &proto::PlayerBasicInfo::PlayerBasicInfo) {
        let p = &mut self.account.profile;
        if !info.nickname.is_empty() {
            p.nickname = Some(info.nickname.clone());
        }
        if info.level != 0 {
            p.level = Some(info.level);
        }
        p.trailblaze_power = Some(info.stamina);
        p.stellar_jade = Some(info.stellar_jade_count as u64);
        p.credits = Some(info.credit_count as u64);
        p.touch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command<M: protobuf::Message>(id: u16, msg: &M) -> GameCommand {
        let data = msg.write_to_bytes().unwrap();
        GameCommand { command_id: id, header_len: 0, data_len: data.len() as u32, proto_header: vec![], proto_data: data }
    }

    fn token(uid: u32) -> GameCommand {
        let mut t = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
        t.uid = uid;
        command(cmd::PlayerGetTokenScRsp, &t)
    }

    fn bag_with_relic(relic_uid: u32) -> GameCommand {
        let mut r = proto::Relic::Relic::new();
        r.unique_id = relic_uid;
        r.tid = 61011;
        r.main_affix_id = 1;
        let mut b = proto::GetBagScRsp::GetBagScRsp::new();
        b.relic_list = vec![r];
        command(cmd::GetBagScRsp, &b)
    }

    fn battle() -> GameCommand {
        let mut req = proto::PVEBattleResultCsReq::PVEBattleResultCsReq::new();
        req.stage_id = 1;
        command(cmd::PVEBattleResultCsReq, &req)
    }

    fn temp_store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("nous-game-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        (Store::new(Some(dir.clone())), dir)
    }

    #[test]
    fn switching_account_replaces_everything_and_keeps_the_old_one() {
        let (store, dir) = temp_store("switch");
        let mut game = GameState::new(store.clone());

        let ch = game.handle(&token(1));
        assert!(ch.account_switched && ch.profile);
        game.handle(&bag_with_relic(10));
        let ch = game.handle(&battle());
        assert!(ch.battle.is_some());
        let rev_a = ch.revision;

        let ch = game.handle(&token(2));
        assert!(ch.account_switched && ch.inventory_reset, "consumers must drop account 1");
        assert!(ch.revision > rev_a, "revisions only grow");
        assert_eq!(game.account.uid, Some(2));
        assert!(game.account.inventory.relics.is_empty());
        assert!(game.account.battles.list.is_empty(), "battles of account 1 don't leak into 2");

        // account 1 was written before the switch, with its relic and battle
        let mut back = GameState::new(store.clone());
        back.handle(&token(1));
        assert!(back.account.inventory.relics.contains_key(&10));
        assert_eq!(back.account.battles.list.len(), 1);

        // same uid again: not a switch
        assert!(!back.handle(&token(1)).account_switched);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn handling_marks_unsaved_instead_of_writing() {
        let (store, dir) = temp_store("dirty");
        let mut game = GameState::new(store);
        game.handle(&token(3));
        let before = game.revision();
        game.handle(&bag_with_relic(1));
        assert!(game.revision() > before);
        assert!(game.take_unsaved().is_some(), "changed: one copy to write");
        assert!(game.take_unsaved().is_none(), "nothing new since");
        std::fs::remove_dir_all(dir).ok();
    }
}
