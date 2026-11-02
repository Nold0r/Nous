//! Game state rebuilt from the decrypted game commands: the player profile
//! (who logged in and their resources). The profile is saved to the app data
//! folder so it survives restarts.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
use tauri::{AppHandle, Manager};

use crate::profile::{PlayerProfile, ProfileSource};
use crate::sniffer::SnifferStatus;

const PROFILE_FILE: &str = "profile.json";

/// App-wide state managed by Tauri.
pub struct Shared {
    pub game: Mutex<GameState>,
    pub status: Mutex<SnifferStatus>,
}

pub struct GameState {
    data_dir: Option<PathBuf>,
    pub profile: PlayerProfile,
}

impl GameState {
    pub fn load(app: &AppHandle) -> Self {
        let data_dir = app.path().app_data_dir().ok();
        let profile = data_dir
            .as_ref()
            .and_then(|d| fs::read_to_string(d.join(PROFILE_FILE)).ok())
            .and_then(|s| serde_json::from_str::<PlayerProfile>(&s).ok())
            .map(|mut p| {
                p.source = ProfileSource::Cached;
                p
            })
            .unwrap_or_else(PlayerProfile::empty);
        Self { data_dir, profile }
    }

    /// Applies one decrypted game command; true if the profile changed.
    pub fn handle(&mut self, command: &GameCommand) -> bool {
        let changed = match command.command_id {
            cmd::PlayerGetTokenScRsp => {
                let Ok(rsp) = command.parse_proto::<proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp>() else {
                    return false;
                };
                if self.profile.uid != Some(rsp.uid) {
                    // another account: forget the old one's numbers
                    self.profile = PlayerProfile::empty();
                }
                self.profile.uid = Some(rsp.uid);
                self.profile.touch();
                true
            }
            cmd::PlayerLoginScRsp => {
                let Ok(rsp) = command.parse_proto::<proto::PlayerLoginScRsp::PlayerLoginScRsp>() else {
                    return false;
                };
                if let Some(info) = rsp.basic_info.as_ref() {
                    self.apply_basic_info(info);
                }
                true
            }
            cmd::PlayerSyncScNotify => {
                let Ok(sync) = command.parse_proto::<proto::PlayerSyncScNotify::PlayerSyncScNotify>() else {
                    return false;
                };
                let Some(info) = sync.basic_info.as_ref() else { return false };
                self.apply_basic_info(info);
                true
            }
            cmd::StaminaInfoScNotify => {
                let Ok(n) = command.parse_proto::<proto::StaminaInfoScNotify::StaminaInfoScNotify>() else {
                    return false;
                };
                self.profile.trailblaze_power = Some(n.stamina);
                self.profile.reserved_trailblaze_power = Some(n.reserve_stamina);
                self.profile.touch();
                true
            }
            cmd::GetPlayerBoardDataScRsp => {
                let Ok(rsp) = command.parse_proto::<proto::GetPlayerBoardDataScRsp::GetPlayerBoardDataScRsp>() else {
                    return false;
                };
                if rsp.current_head_icon_id == 0 {
                    return false;
                }
                self.profile.avatar_id = Some(rsp.current_head_icon_id);
                self.profile.touch();
                true
            }
            _ => false,
        };

        if changed {
            self.save_profile();
        }
        changed
    }

    fn apply_basic_info(&mut self, info: &proto::PlayerBasicInfo::PlayerBasicInfo) {
        let p = &mut self.profile;
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

    fn save_profile(&self) {
        let Some(dir) = &self.data_dir else { return };
        if fs::create_dir_all(dir).is_ok() {
            if let Ok(json) = serde_json::to_string_pretty(&self.profile) {
                if let Err(e) = fs::write(dir.join(PROFILE_FILE), json) {
                    log::warn!("cannot save {PROFILE_FILE}: {e}");
                }
            }
        }
    }
}
