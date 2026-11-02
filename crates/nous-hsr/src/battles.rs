use nous_core::battles::*;
use reliquary::network::command::{GameCommand, command_id as cmd, proto};
use std::collections::BTreeMap;
pub fn decode(command: &GameCommand, now: i64) -> Option<Battle> {
    if command.command_id != cmd::PVEBattleResultCsReq {
        return None;
    }
    let req = command
        .parse_proto::<proto::PVEBattleResultCsReq::PVEBattleResultCsReq>()
        .ok()?;
    Some(Battle::from_proto(&req, now))
}

trait BattleProto {
    fn from_proto(req: &proto::PVEBattleResultCsReq::PVEBattleResultCsReq, now: i64) -> Self;
}
impl BattleProto for Battle {
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

trait AvatarProto {
    fn from_proto(a: &proto::AvatarBattleInfo::AvatarBattleInfo, battle_av: f64) -> Self;
}
impl AvatarProto for BattleAvatar {
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
            let e = max_hit_by_type
                .entry(p.attack_type.clone())
                .or_insert(0.0_f64);
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
            dpav: if battle_av > 0.0 {
                a.total_damage / battle_av
            } else {
                0.0
            },
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
        let b = decode(&command, 1).expect("battle parsed");
        battles.push(b.clone());
        assert_eq!(b.result, "win");
        assert_eq!(b.turns, 7);
        assert_eq!(b.avatars[0].damage, 150_000.0);
        assert_eq!(
            b.avatars[0].damage_by_type["Ultra"], 240_000.0,
            "same attack type is summed"
        );
        assert_eq!(
            b.avatars[0].dpav, 500.0,
            "damage / action value of the fight"
        );
        assert_eq!(b.actions[0].skill_index, 2);
        assert_eq!(battles.list.len(), 1);
    }
}
