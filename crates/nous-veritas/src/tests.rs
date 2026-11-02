#![cfg(test)]

use super::*;
use serde_json::json;

#[test]
fn recognizes_actual_wire_events_but_ignores_unknown_names() {
    let mut engine = Engine::default();
    assert!(!engine.apply("Heartbeat", json!({})).unwrap());
    assert!(
        engine
            .apply(
                "OnBattleBegin",
                json!({"max_waves":2,"max_cycles":30,"stage_id":1001})
            )
            .unwrap()
    );
}

fn begin(engine: &mut Engine, lineup_first: bool) {
    let lineup = json!({"avatars":[{"id":1002,"name":"Dan Heng"},{"id":1003,"name":"Himeko"}]});
    let start = json!({"max_waves":2,"max_cycles":30,"stage_id":1001});
    let events = if lineup_first {
        vec![("OnSetBattleLineup", lineup), ("OnBattleBegin", start)]
    } else {
        vec![("OnBattleBegin", start), ("OnSetBattleLineup", lineup)]
    };
    for (name, data) in events {
        engine.apply(name, data).unwrap();
    }
}

fn turn(engine: &mut Engine, damage: f64, av: f64) {
    engine
        .apply(
            "OnTurnBegin",
            json!({"action_value":av,"turn_owner":{"uid":1002,"team":"Player"}}),
        )
        .unwrap();
    engine.apply("OnUseSkill", json!({"avatar":{"uid":1002,"team":"Player"},"skill":{"name":"Torrent","type":"BPSkill","skill_config_id":100202}})).unwrap();
    engine.apply("OnDamage", json!({"attacker":{"uid":1002,"team":"Player"},"damage":damage,"overkill_damage":5,"type":"Skill"})).unwrap();
    engine.apply("OnTurnEnd", json!({"turn_info":{"action_value":av,"cycle":29,"wave":1,"avatars_turn_damage":[damage,0],"total_damage":damage}})).unwrap();
}

fn end(engine: &mut Engine) {
    let battle = engine.snapshot().battle.unwrap();
    let turns: Vec<_> = battle.turn_history.iter().map(|turn| json!({"action_value":turn.action_value,"cycle":turn.cycle,"wave":turn.wave,"avatars_turn_damage":turn.avatars_turn_damage,"total_damage":turn.total_damage})).collect();
    engine.apply("OnBattleEnd", json!({"avatars":[{"id":1002,"name":"Dan Heng"},{"id":1003,"name":"Himeko"}],"turn_history":turns,"av_history":[],"turn_count":battle.turn_history.len(),"total_damage":battle.total_damage,"action_value":battle.action_value,"cycle":29,"wave":1,"stage_id":0})).unwrap();
}

#[test]
fn begin_and_lineup_merge_in_both_orders_without_losing_metadata() {
    for lineup_first in [true, false] {
        let mut engine = Engine::default();
        begin(&mut engine, lineup_first);
        let state = engine.snapshot();
        let battle = state.battle.unwrap();
        assert!(state.active);
        assert!(!battle.partial);
        assert_eq!(
            (battle.stage_id, battle.max_waves, battle.max_cycles),
            (1001, 2, 30)
        );
        assert_eq!(battle.avatars.len(), 2);
        assert!(engine.history().is_empty());
    }
}

#[test]
fn damage_aggregates_live_skills_overkill_and_equal_av_turns() {
    let mut engine = Engine::default();
    begin(&mut engine, true);
    turn(&mut engine, 60.0, 75.0);
    turn(&mut engine, 40.0, 75.0);
    let battle = engine.snapshot().battle.unwrap();
    assert_eq!(battle.total_damage, 100.0);
    assert_eq!(battle.avatars[0].damage, 100.0);
    assert_eq!(battle.avatars[0].overkill, 10.0);
    assert_eq!(battle.avatars[0].by_type["Skill"], 100.0);
    assert_eq!(battle.skills[0].damage, 60.0);
    assert_eq!(battle.skills[1].details[0].overkill_damage, 5.0);
    assert_eq!(battle.skills[1].turn_index, 2);
    assert_eq!(battle.entity_turns.len(), 2);
    assert_eq!(battle.turn_history.len(), 2);
    assert_eq!(battle.av_history.len(), 1);
    assert_eq!(battle.av_history[0].avatars_turn_damage, vec![100.0, 0.0]);
    assert_eq!(battle.av_history[0].total_damage, 100.0);
    let json = serde_json::to_value(&battle).unwrap();
    assert_eq!(json["turnHistory"][0]["avatarsTurnDamage"][0], 60.0);
}

#[test]
fn current_properties_and_enemy_formation_are_preserved() {
    let mut engine = Engine::default();
    begin(&mut engine, false);
    for (uid, id) in [(9001, 77), (9002, 78)] {
        engine.apply("OnInitializeEnemy", json!({"enemy":{"id":id,"uid":uid,"name":"Reaver","base_stats":{"properties":{"CurrentHP":1000,"MaxHP":1000,"Level":85}}}})).unwrap();
    }
    engine.apply("OnUpdateTeamFormation",json!({"team":"Enemy","entities":[{"uid":9002,"team":"Enemy"},{"uid":9001,"team":"Enemy"}]})).unwrap();
    engine.apply("OnStatChange",json!({"entity":{"uid":9002,"team":"Enemy"},"property":{"type":"CurrentHP","value":250}})).unwrap();
    engine.apply("OnStatChange",json!({"entity":{"uid":1002,"team":"Player"},"property":{"type":"ActionDelay","value":32}})).unwrap();
    engine.apply("OnEntityDefeated",json!({"killer":{"uid":1002,"team":"Player"},"entity_defeated":{"uid":9001,"team":"Enemy"}})).unwrap();
    let battle = engine.snapshot().battle.unwrap();
    assert_eq!(battle.enemies[0].uid, 9002);
    assert_eq!(battle.enemies[0].properties["CurrentHP"], 250.0);
    assert!(battle.enemies[1].dead);
    assert_eq!(battle.avatars[0].properties["ActionDelay"], 32.0);
}

#[test]
fn malformed_event_is_atomic_and_unknown_event_does_not_change_revision() {
    let mut engine = Engine::default();
    begin(&mut engine, false);
    let before = serde_json::to_value(engine.snapshot()).unwrap();
    assert!(
        engine
            .apply(
                "OnDamage",
                json!({"attacker":{"uid":1002,"team":"Player"},"damage":"bad"})
            )
            .is_err()
    );
    assert!(!engine.apply("OnKill", json!({})).unwrap());
    assert_eq!(serde_json::to_value(engine.snapshot()).unwrap(), before);
}

#[test]
fn disconnect_archives_interrupted_battle_only_once() {
    let mut engine = Engine::default();
    engine.set_connected(true);
    begin(&mut engine, true);
    turn(&mut engine, 60.0, 75.0);
    engine.set_connected(false);
    engine.set_connected(false);
    engine.finish_interrupted();
    let state = engine.snapshot();
    assert!(!state.connected);
    assert!(!state.active);
    assert_eq!(engine.history().len(), 1);
    assert!(engine.history()[0].interrupted);
    assert!(engine.history()[0].ended_at.is_some());
    assert_eq!(engine.history()[0].total_damage, 60.0);
}

#[test]
fn battle_end_retains_live_details_and_nonzero_stage_without_duplicate_history() {
    let mut engine = Engine::default();
    begin(&mut engine, true);
    turn(&mut engine, 60.0, 75.0);
    end(&mut engine);
    end(&mut engine);
    let state = engine.snapshot();
    assert!(!state.active);
    assert_eq!(engine.history().len(), 1);
    let battle = engine.battle(state.battle.unwrap().id).unwrap();
    assert_eq!(battle.stage_id, 1001);
    assert_eq!(battle.skills.len(), 1);
    assert_eq!(battle.avatars[0].overkill, 5.0);
    assert_eq!(battle.avatars[0].by_type["Skill"], 60.0);
    assert!(!battle.partial);
}

#[test]
fn late_join_is_marked_partial_and_end_recovers_avatar_totals() {
    let mut engine = Engine::default();
    engine.apply("OnBattleEnd",json!({"avatars":[{"id":1002,"name":"Dan Heng"}],"turn_history":[{"action_value":75,"wave":1,"cycle":29,"avatars_turn_damage":[123],"total_damage":123}],"av_history":[],"turn_count":1,"total_damage":123,"action_value":75,"cycle":29,"wave":1,"stage_id":1001})).unwrap();
    let battle = engine.history()[0].clone();
    assert!(battle.partial);
    assert_eq!(battle.avatars[0].damage, 123.0);
    assert_eq!(battle.total_damage, 123.0);
}

#[test]
fn new_battle_interrupts_prior_battle_and_retention_restores_ids() {
    let mut engine = Engine::default();
    for _ in 0..23 {
        begin(&mut engine, true);
    }
    assert_eq!(engine.history().len(), 20);
    let history = engine.history().to_vec();
    let max_id = history.iter().map(|b| b.id).max().unwrap();
    let mut restored = Engine::default();
    restored.restore_history(history);
    begin(&mut restored, false);
    assert!(restored.snapshot().battle.unwrap().id > max_id);
    assert_eq!(restored.snapshot().history.len(), 20);
}

#[test]
fn only_fully_eligible_overlay_is_visible() {
    assert!(visible(true, true, true, true, false, false));
    for flags in [
        [false, true, true, true, false, false],
        [true, false, true, true, false, false],
        [true, true, false, true, false, false],
        [true, true, true, false, false, false],
        [true, true, true, true, true, false],
        [true, true, true, true, false, true],
    ] {
        assert!(!visible(
            flags[0], flags[1], flags[2], flags[3], flags[4], flags[5]
        ));
    }
}

#[test]
fn exports_source_json_names_and_complete_csv_rows_with_escaping() {
    let mut engine = Engine::default();
    begin(&mut engine, true);
    turn(&mut engine, 60.0, 75.0);
    end(&mut engine);
    let mut battle = engine.history()[0].clone();
    battle.avatars[0].name = "Dan, \"Heng\"".into();
    let exported = battle.export_json("0.2.57");
    assert_eq!(exported["totalDamage"], 60.0);
    assert_eq!(exported["damagePerAV"], 0.8);
    assert_eq!(exported["version"], "0.2.57");
    assert_eq!(exported["turnHistory"][0]["avatarId"], -1);
    assert_eq!(
        exported["skillHistory"][0]["damageDetail"][0]["overkill_damage"],
        5.0
    );
    assert!(exported["avatarDetail"]["1002"]["stats"].is_object());
    assert!(exported["enemyDetail"].is_object());
    let csv = battle.export_csv();
    assert_eq!(
        csv.lines().next().unwrap(),
        "data_type,character_name,character_id,total_damage,damage_percentage,dpav,primary_skill_usage_count,turns_taken,average_damage_per_turn,max_single_turn_damage,first_turn_number,last_turn_number,turn_order,turn_battle_id,wave,cycle,action_value,skill_name,skill_type,skill_type_name,skill_damage,cumulative_damage,cumulative_character_damage,skill_damage_percentage"
    );
    assert!(csv.contains("\"Dan, \"\"Heng\"\"\""));
    assert_eq!(csv.lines().count(), 4);
    assert!(csv.contains("skill_detail,"));
}

#[test]
fn missing_damage_is_detected_at_turn_end_and_repairs_live_totals() {
    let mut engine = Engine::default();
    begin(&mut engine, false);
    engine.apply("OnTurnEnd",json!({"turn_info":{"action_value":30,"wave":1,"cycle":29,"avatars_turn_damage":[25,10],"total_damage":35}})).unwrap();
    let battle = engine.snapshot().battle.unwrap();
    assert!(battle.partial);
    assert_eq!(battle.total_damage, 35.0);
    assert_eq!(battle.avatars[0].damage, 25.0);
    assert_eq!(battle.avatars[1].damage, 10.0);
}

#[test]
fn disconnect_finalizes_active_state_even_without_preceding_connection_notification() {
    let mut engine = Engine::default();
    begin(&mut engine, true);
    engine.set_connected(false);
    assert!(!engine.snapshot().active);
    assert_eq!(engine.history().len(), 1);
}

#[test]
fn retained_details_are_bounded_and_truncation_marks_partial() {
    let mut engine = Engine::default();
    begin(&mut engine, true);
    let turn = json!({"turn_info":{"action_value":30,"wave":1,"cycle":29,"avatars_turn_damage":[0,0],"total_damage":0}});
    for _ in 0..4100 {
        engine
            .apply(
                "OnTurnBegin",
                json!({"action_value":30,"turn_owner":{"uid":1002,"team":"Player"}}),
            )
            .unwrap();
        engine.apply("OnUseSkill",json!({"avatar":{"uid":1002,"team":"Player"},"skill":{"name":"Torrent","type":"BPSkill","skill_config_id":100202}})).unwrap();
        engine.apply("OnTurnEnd", turn.clone()).unwrap();
    }
    let battle = engine.snapshot().battle.unwrap();
    assert!(battle.partial);
    assert!(battle.turn_history.len() <= 4096);
    assert!(battle.entity_turns.len() <= 4096);
    assert!(battle.skills.len() <= 4096);
}

#[test]
fn protocol_connection_errors_and_cycle_updates_reach_snapshot() {
    let mut engine = Engine::default();
    engine
        .apply("Connected", json!({"version":"0.2.57"}))
        .unwrap();
    assert!(engine.snapshot().connected);
    assert!(engine.snapshot().diagnostics[0].contains("0.2.57"));
    engine.apply("Error", json!({"msg":"test error"})).unwrap();
    begin(&mut engine, false);
    engine.apply("OnUpdateWave", json!({"wave":2})).unwrap();
    engine.apply("OnUpdateCycle", json!({"cycle":28})).unwrap();
    let state = engine.snapshot();
    assert_eq!(state.diagnostics[1], "test error");
    let battle = state.battle.unwrap();
    assert_eq!((battle.wave, battle.cycle), (2, 28));
}

#[test]
fn restoring_history_exposes_latest_battle_without_resuming_it() {
    let mut engine = Engine::default();
    begin(&mut engine, true);
    turn(&mut engine, 60.0, 75.0);
    end(&mut engine);
    let mut restored = Engine::default();
    restored.restore_history(engine.history().to_vec());
    let state = restored.snapshot();
    assert!(!state.active);
    assert!(!state.connected);
    assert_eq!(state.battle.unwrap().total_damage, 60.0);
}

#[test]
fn source_json_uses_tracked_stat_subset_and_omits_empty_stat_history() {
    let battle = Battle {
        avatars: vec![Combatant {
            id: 1,
            name: "test".into(),
            ..Combatant::default()
        }],
        ..Battle::default()
    };
    assert_eq!(
        battle.export_json("0.2.57")["avatarDetail"]["1"]["statsHistory"],
        json!([])
    );
    let mut battle = battle;
    battle.avatars[0].properties.insert("Attack".into(), 123.0);
    battle.avatars[0].properties.insert("Level".into(), 85.0);
    let exported = battle.export_json("0.2.57");
    assert_eq!(
        exported["avatarDetail"]["1"]["stats"],
        json!({"Attack":123.0})
    );
    assert_eq!(
        exported["avatarDetail"]["1"]["statsHistory"][0]["stats"],
        json!({"Attack":123.0})
    );
}
