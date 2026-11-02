use serde::{Deserialize, Serialize};
use tauri::State;

use crate::game::Shared;

/// Game caps, shown even before an account is linked; live data overrides them.
const TRAILBLAZE_POWER_MAX: u32 = 300;
const RESERVED_TRAILBLAZE_POWER_MAX: u32 = 2400;

/// Where the profile numbers came from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileSource {
    /// Read from the game traffic during this session.
    Live,
    /// Saved from an earlier session, the game hasn't been seen since start.
    Cached,
    /// No account linked yet: every value is empty.
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub source: ProfileSource,
    pub nickname: Option<String>,
    pub uid: Option<u32>,
    pub level: Option<u32>,
    /// Profile picture: head icon id from the game (see src/lib/icons.ts for the file).
    pub avatar_id: Option<u32>,
    pub stellar_jade: Option<u64>,
    pub credits: Option<u64>,
    pub trailblaze_power: Option<u32>,
    pub trailblaze_power_max: Option<u32>,
    pub reserved_trailblaze_power: Option<u32>,
    pub reserved_trailblaze_power_max: Option<u32>,
    /// Unix timestamp (seconds) of the last update.
    pub updated_at: Option<i64>,
}

impl PlayerProfile {
    /// Profile of an account that hasn't been linked yet.
    pub fn empty() -> Self {
        Self {
            source: ProfileSource::None,
            nickname: None,
            uid: None,
            level: None,
            avatar_id: None,
            stellar_jade: None,
            credits: None,
            trailblaze_power: None,
            trailblaze_power_max: Some(TRAILBLAZE_POWER_MAX),
            reserved_trailblaze_power: None,
            reserved_trailblaze_power_max: Some(RESERVED_TRAILBLAZE_POWER_MAX),
            updated_at: None,
        }
    }

    /// Marks the profile as freshly read from the game.
    pub fn touch(&mut self) {
        self.source = ProfileSource::Live;
        self.updated_at = Some(now());
    }
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_profile(shared: State<'_, Shared>) -> PlayerProfile {
    shared.game.lock().unwrap().profile().clone()
}
