//! Game state rebuilt from the decrypted game commands: the current account
//! (profile and inventory). Stored in the one account format, see account.rs.

use std::sync::Mutex;

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
use tauri::{AppHandle, Manager};

use crate::account::{Account, Store};
use crate::inventory::InventoryChanges;
use crate::profile::{PlayerProfile, now};
use crate::sniffer::SnifferStatus;

/// App-wide state managed by Tauri.
pub struct Shared {
    pub game: Mutex<GameState>,
    pub status: Mutex<SnifferStatus>,
}

/// What a command changed, so the sniffer knows which events to emit.
#[derive(Debug, Default)]
pub struct Changes {
    pub profile: bool,
    /// The whole inventory was replaced (login, other account).
    pub inventory_reset: bool,
    pub inventory: InventoryChanges,
}

pub struct GameState {
    store: Store,
    pub account: Account,
}

impl GameState {
    pub fn load(app: &AppHandle) -> Self {
        let store = Store::new(app.path().app_data_dir().ok());
        let account = store.load_current();
        Self { store, account }
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
                        self.account = self.store.load_or_new(rsp.uid);
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

        if ch.profile || ch.inventory_reset || !ch.inventory.is_empty() {
            self.store.save(&self.account);
        }
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
