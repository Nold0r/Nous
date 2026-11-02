//! Veritas 0.2.57 payloads. These deliberately retain the source's snake_case names.
use crate::{Entity, Team};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize)]
pub struct Avatar {
    pub id: u32,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Property {
    pub value: f64,
    pub r#type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub r#type: String,
    pub skill_config_id: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BattleStats {
    pub properties: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Enemy {
    pub id: u32,
    pub uid: u32,
    pub name: String,
    pub base_stats: BattleStats,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TurnInfo {
    pub action_value: f64,
    pub cycle: u32,
    pub wave: u32,
    pub avatars_turn_damage: Vec<f64>,
    pub total_damage: f64,
}

impl From<TurnInfo> for crate::TurnInfo {
    fn from(value: TurnInfo) -> Self {
        Self {
            action_value: value.action_value,
            cycle: value.cycle,
            wave: value.wave,
            avatars_turn_damage: value.avatars_turn_damage,
            total_damage: value.total_damage,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum Event {
    Connected {
        version: String,
    },
    Error {
        msg: String,
    },
    OnBattleBegin {
        max_waves: u32,
        max_cycles: u32,
        stage_id: u32,
    },
    OnSetBattleLineup {
        avatars: Vec<Avatar>,
    },
    OnDamage {
        attacker: Entity,
        damage: f64,
        overkill_damage: f64,
        r#type: String,
    },
    OnTurnBegin {
        action_value: f64,
        turn_owner: Option<Entity>,
    },
    OnTurnEnd {
        turn_info: TurnInfo,
    },
    OnEntityDefeated {
        killer: Entity,
        entity_defeated: Entity,
    },
    OnUseSkill {
        avatar: Entity,
        skill: Skill,
    },
    OnUpdateWave {
        wave: u32,
    },
    OnUpdateCycle {
        cycle: u32,
    },
    OnStatChange {
        entity: Entity,
        property: Property,
    },
    OnUpdateTeamFormation {
        entities: Vec<Entity>,
        team: Team,
    },
    OnInitializeEnemy {
        enemy: Enemy,
    },
    OnBattleEnd {
        avatars: Vec<Avatar>,
        turn_history: Vec<TurnInfo>,
        av_history: Vec<TurnInfo>,
        turn_count: usize,
        total_damage: f64,
        action_value: f64,
        cycle: u32,
        wave: u32,
        stage_id: u32,
    },
}

pub fn decode(name: &str, data: serde_json::Value) -> Result<Option<Event>, String> {
    if !matches!(
        name,
        "Connected"
            | "Error"
            | "OnBattleBegin"
            | "OnSetBattleLineup"
            | "OnDamage"
            | "OnTurnBegin"
            | "OnTurnEnd"
            | "OnEntityDefeated"
            | "OnUseSkill"
            | "OnUpdateWave"
            | "OnUpdateCycle"
            | "OnStatChange"
            | "OnUpdateTeamFormation"
            | "OnInitializeEnemy"
            | "OnBattleEnd"
    ) {
        return Ok(None);
    }
    serde_json::from_value(serde_json::json!({"event":name,"data":data}))
        .map(Some)
        .map_err(|error| format!("{name}: {error}"))
}
