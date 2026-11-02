//! Battle results. At the end of every fight the game client reports its own
//! statistics to the server (`PVEBattleResultCsReq.stt`): damage, healing,
//! breaks, skill uses per character and per enemy. Nous reads that report from
//! the traffic; the fight itself runs in the client, so there is no hit-by-hit
//! stream on the wire, only this summary.

use std::collections::BTreeMap;

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
use serde::{Deserialize, Serialize};

/// Older battles are dropped beyond this many.
const KEEP: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleAction {
    pub turn: u32,
    pub actor: u32,
    pub target: u32,
    pub op_type: u32,
    pub skill_index: u32,
    pub counter: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleMonster {
    pub monster_id: u32,
    pub wave: u32,
    pub damage: f64,
    pub damage_taken: f64,
    pub turns: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Battles {
    /// Newest last.
    pub list: Vec<Battle>,
}

impl Battles {
    /// The battle report, if this command is one.
    pub fn handle(&mut self, command: &GameCommand, now: i64) -> Option<Battle> {
        if command.command_id != cmd::PVEBattleResultCsReq {
            return None;
        }
        let req = command.parse_proto::<proto::PVEBattleResultCsReq::PVEBattleResultCsReq>().ok()?;
        let battle = Battle::from_proto(&req, now);
        self.list.push(battle.clone());
        if self.list.len() > KEEP {
            self.list.drain(..self.list.len() - KEEP);
        }
        Some(battle)
    }
}

impl Battle {
    fn from_proto(req: &proto::PVEBattleResultCsReq::PVEBattleResultCsReq, now: i64) -> Self {
        use proto::BattleEndStatus::BattleEndStatus as End;
        let stt = req.stt.as_ref().cloned().unwrap_or_default();
        let result = match req.end_status.enum_value() {
            Ok(End::BATTLE_END_WIN) => "win",
            Ok(End::BATTLE_END_LOSE) => "lose",
            Ok(End::BATTLE_END_QUIT) => "quit",
            _ => "none",
        };
        Self {
            battle_id: req.battle_id,
            stage_id: req.stage_id,
            result: result.into(),
            at: now,
            cost_time: stt.cost_time,
            turns: stt.total_battle_turns,
            auto_turns: stt.total_auto_turns,
            ultimates: stt.ultra_cnt,
            rounds: stt.round_cnt,
            challenge_score: stt.challenge_score,
            action_value: stt.total_delay_cumulate,
            avatars: stt
                .battle_avatar_list
                .iter()
                .map(|a| BattleAvatar::from_proto(a, stt.total_delay_cumulate))
                .collect(),
            actions: req
                .op_list
                .iter()
                .map(|o| BattleAction {
                    turn: o.turn_counter,
                    actor: o.action_entity_id,
                    target: o.target_entity_id,
                    op_type: o.op_type,
                    skill_index: o.skill_index,
                    counter: o.operation_counter,
                })
                .collect(),
            monsters: stt
                .monster_list
                .iter()
                .map(|m| BattleMonster {
                    monster_id: m.monster_id,
                    wave: m.wave,
                    damage: m.total_damage,
                    damage_taken: m.total_damage_taken,
                    turns: m.total_turns,
                })
                .collect(),
        }
    }
}

impl BattleAvatar {
    fn from_proto(a: &proto::AvatarBattleInfo::AvatarBattleInfo, battle_av: f64) -> Self {
        let by_type = |list: &[proto::AttackDamageProperty::AttackDamageProperty]| {
            let mut map = BTreeMap::new();
            for p in list {
                *map.entry(p.attack_type.clone()).or_insert(0.0) += p.damage;
            }
            map
        };
        let mut max_hit_by_type = BTreeMap::new();
        for p in &a.attack_type_max_damage {
            let e = max_hit_by_type.entry(p.attack_type.clone()).or_insert(0.0_f64);
            *e = e.max(p.damage);
        }
        let mut skill_uses = BTreeMap::new();
        for s in &a.skill_times {
            *skill_uses.entry(s.skill_type.clone()).or_insert(0) += s.skill_use_count;
        }
        Self {
            avatar_id: a.id,
            level: a.avatar_level,
            eidolon: a.avatar_rank,
            assist_uid: a.assist_uid,
            turns: a.total_turns,
            damage: a.total_damage,
            break_damage: a.total_break_damage,
            heal: a.total_heal,
            shield: a.total_shield,
            damage_taken: a.total_damage_taken,
            damage_by_type: by_type(&a.attack_type_damage),
            max_hit_by_type,
            skill_uses,
            breaks: a.break_times,
            deaths: a.die_times,
            sp_used: a.total_sp_cost,
            sp_gained: a.total_sp_add,
            action_value: a.delay_cumulate,
            dpav: if battle_av > 0.0 { a.total_damage / battle_av } else { 0.0 },
            relics: a.relics.iter().map(|r| r.unique_id).collect(),
            light_cone: a.avatar_equipment.first().map(|e| e.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use protobuf::Message;

    use super::*;

    #[test]
    fn reads_the_client_battle_report() {
        let mut dmg = proto::AttackDamageProperty::AttackDamageProperty::new();
        dmg.attack_type = "Ultra".into();
        dmg.damage = 120_000.0;
        let mut avatar = proto::AvatarBattleInfo::AvatarBattleInfo::new();
        avatar.id = 1310;
        avatar.total_damage = 150_000.0;
        avatar.attack_type_damage = vec![dmg.clone(), dmg];
        let mut stt = proto::BattleStatistics::BattleStatistics::new();
        stt.battle_avatar_list = vec![avatar];
        stt.total_battle_turns = 7;
        stt.total_delay_cumulate = 300.0;
        let mut req = proto::PVEBattleResultCsReq::PVEBattleResultCsReq::new();
        req.stage_id = 30_101;
        req.end_status = proto::BattleEndStatus::BattleEndStatus::BATTLE_END_WIN.into();
        req.stt = Some(stt).into();
        let mut op = proto::BattleOp::BattleOp::new();
        op.turn_counter = 1;
        op.skill_index = 2;
        req.op_list = vec![op];

        let command = GameCommand {
            command_id: cmd::PVEBattleResultCsReq,
            header_len: 0,
            data_len: 0,
            proto_header: vec![],
            proto_data: req.write_to_bytes().unwrap(),
        };
        let mut battles = Battles::default();
        let b = battles.handle(&command, 1).expect("battle parsed");
        assert_eq!(b.result, "win");
        assert_eq!(b.turns, 7);
        assert_eq!(b.avatars[0].damage, 150_000.0);
        assert_eq!(b.avatars[0].damage_by_type["Ultra"], 240_000.0, "same attack type is summed");
        assert_eq!(b.avatars[0].dpav, 500.0, "damage / action value of the fight");
        assert_eq!(b.actions[0].skill_index, 2);
        assert_eq!(battles.list.len(), 1);
    }
}
