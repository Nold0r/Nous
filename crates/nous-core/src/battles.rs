//! Battle results. At the end of every fight the game client reports its own
//! statistics to the server (`PVEBattleResultCsReq.stt`): damage, healing,
//! breaks, skill uses per character and per enemy. Nous reads that report from
//! the traffic; the fight itself runs in the client, so there is no hit-by-hit
//! stream on the wire, only this summary.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Older battles are dropped beyond this many.
const KEEP: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Battle {
    pub battle_id: u32,
    /// StageConfig id (which fight).
    pub stage_id: u32,
    /// win / lose / quit / none
    pub result: String,
    /// Unix seconds Nous saw the result.
    pub at: i64,
    /// Seconds, as the client measured them.
    pub cost_time: f64,
    pub turns: u32,
    pub auto_turns: u32,
    pub ultimates: u32,
    /// Waves/cycles (MoC counts cycles here).
    pub rounds: u32,
    pub challenge_score: u32,
    /// Action value the whole fight took (`total_delay_cumulate`).
    pub action_value: f64,
    pub avatars: Vec<BattleAvatar>,
    pub monsters: Vec<BattleMonster>,
    /// Client's action log (`op_list`): who acted on whom with which skill,
    /// in order. Entity ids are battle-local; semantics still to be verified
    /// on real fights.
    pub actions: Vec<BattleAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct BattleAction {
    pub turn: u32,
    pub actor: u32,
    pub target: u32,
    pub op_type: u32,
    pub skill_index: u32,
    pub counter: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct BattleAvatar {
    pub avatar_id: u32,
    pub level: u32,
    pub eidolon: u32,
    /// Support character borrowed from another player (their uid), 0 otherwise.
    pub assist_uid: u32,
    pub turns: u32,
    pub damage: f64,
    pub break_damage: f64,
    pub heal: f64,
    pub shield: f64,
    pub damage_taken: f64,
    /// Attack type (Normal, BPSkill, Ultra, Insert, DOT, …) -> damage.
    pub damage_by_type: BTreeMap<String, f64>,
    /// Attack type -> biggest single hit.
    pub max_hit_by_type: BTreeMap<String, f64>,
    /// Skill type -> times used.
    pub skill_uses: BTreeMap<String, u32>,
    pub breaks: u32,
    pub deaths: u32,
    pub sp_used: f64,
    pub sp_gained: u32,
    /// Action value this character accumulated (`delay_cumulate`).
    pub action_value: f64,
    /// Damage per action value of the whole fight, Veritas' DPAV; 0 if unknown.
    pub dpav: f64,
    /// Relic unique ids worn in this fight.
    pub relics: Vec<u32>,
    /// Light cone (EquipmentConfig id), if any.
    pub light_cone: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct BattleMonster {
    pub monster_id: u32,
    pub wave: u32,
    pub damage: f64,
    pub damage_taken: f64,
    pub turns: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Battles {
    /// Newest last.
    pub list: Vec<Battle>,
}

impl Battles {
    pub fn push(&mut self, battle: Battle) {
        self.list.push(battle);
        if self.list.len() > KEEP {
            self.list.drain(..self.list.len() - KEEP);
        }
    }
}
