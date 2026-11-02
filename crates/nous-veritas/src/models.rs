use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entity {
    pub uid: u32,
    pub team: Team,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TurnInfo {
    pub action_value: f64,
    pub cycle: u32,
    pub wave: u32,
    pub avatars_turn_damage: Vec<f64>,
    pub total_damage: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Combatant {
    pub id: u32,
    pub name: String,
    pub damage: f64,
    pub overkill: f64,
    pub by_type: BTreeMap<String, f64>,
    pub properties: BTreeMap<String, f64>,
    pub dead: bool,
    #[serde(default)]
    pub killer_uid: Option<u32>,
    #[serde(default)]
    pub stats_history: Vec<StatsObservation>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnemyState {
    pub uid: u32,
    pub id: u32,
    pub name: String,
    pub properties: BTreeMap<String, f64>,
    pub dead: bool,
    #[serde(default)]
    pub killer_uid: Option<u32>,
    #[serde(default)]
    pub stats_history: Vec<StatsObservation>,
}

/// A property snapshot observed on the wire, indexed like the source turnHistory.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsObservation {
    pub turn_index: usize,
    pub properties: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageDetail {
    pub damage: f64,
    pub overkill_damage: f64,
    pub r#type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillRecord {
    pub avatar_id: u32,
    pub name: String,
    pub skill_type: String,
    pub skill_config_id: i64,
    pub damage: f64,
    pub details: Vec<DamageDetail>,
    pub turn_index: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityTurn {
    pub owner: Entity,
    pub action_value: f64,
    pub wave: u32,
    pub cycle: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Battle {
    pub id: u64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub interrupted: bool,
    pub partial: bool,
    pub stage_id: u32,
    pub avatars: Vec<Combatant>,
    pub enemies: Vec<EnemyState>,
    pub turn_history: Vec<TurnInfo>,
    pub av_history: Vec<TurnInfo>,
    pub skills: Vec<SkillRecord>,
    pub entity_turns: Vec<EntityTurn>,
    #[serde(default)]
    pub entity_turn_offset: usize,
    pub total_damage: f64,
    pub action_value: f64,
    pub wave: u32,
    pub cycle: u32,
    pub max_waves: u32,
    pub max_cycles: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleSummary {
    pub id: u64,
    pub stage_id: u32,
    pub total_damage: f64,
    pub action_value: f64,
    pub ended_at: Option<i64>,
    pub interrupted: bool,
    pub partial: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub revision: u64,
    pub connected: bool,
    pub active: bool,
    pub battle: Option<Battle>,
    pub history: Vec<BattleSummary>,
    pub diagnostics: Vec<String>,
}
