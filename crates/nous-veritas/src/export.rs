use crate::{Battle, EntityTurn, StatsObservation};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub fn dpav(damage: f64, action_value: f64) -> f64 {
    if action_value > 0.0 {
        damage / action_value
    } else {
        damage
    }
}

impl Battle {
    /// The original Veritas export schema, independently of our camelCase UI DTOs.
    pub fn export_json(&self, version: &str) -> Value {
        let lineup: Vec<_> = self
            .avatars
            .iter()
            .map(|avatar| json!({"avatarId":avatar.id,"isDie":avatar.dead}))
            .collect();
        let mut turns = vec![
            json!({"avatarId":-1,"actionValue":0.0,"waveIndex":self.wave,"cycleIndex":self.max_cycles}),
        ];
        turns.extend(self.entity_turns.iter().map(|turn| json!({"avatarId":turn.owner.uid as i32,"actionValue":turn.action_value,"waveIndex":turn.wave,"cycleIndex":turn.cycle})));
        let skills: Vec<_> = self.skills.iter().map(|skill| {
            let details: Vec<_> = skill.details.iter().map(|detail| json!({"damage":detail.damage,"overkill_damage":detail.overkill_damage,"type":detail.r#type})).collect();
            json!({"avatarId":skill.avatar_id,"damageDetail":details,"totalDamage":skill.damage,
                "skillType":skill.skill_type,"skillName":skill.name,"turnBattleId":skill.turn_index})
        }).collect();
        let avatars: BTreeMap<_, _> = self
            .avatars
            .iter()
            .map(|avatar| {
                let stats = tracked_stats(&avatar.properties);
                (
                    avatar.id.to_string(),
                    json!({
                        "id":avatar.id,"isDie":avatar.dead,"killer_uid":avatar.killer_uid.map(i64::from).unwrap_or(-1),"stats":stats,
                        "statsHistory":stats_history(&avatar.stats_history)
                    }),
                )
            })
            .collect();
        let enemies: BTreeMap<_,_> = self.enemies.iter().enumerate().map(|(index,enemy)| {
            let stats = tracked_stats(&enemy.properties);
            (enemy.uid.to_string(),json!({
            "id":enemy.id,"isDie":enemy.dead,"killer_uid":enemy.killer_uid.map(i64::from).unwrap_or(-1),"positionIndex":index,"waveIndex":self.wave,
            "name":enemy.name,"maxHP":enemy.properties.get("MaxHP").copied().unwrap_or(0.0),
            "level":enemy.properties.get("Level").copied().unwrap_or(0.0) as u32,"stats":stats,
            "statsHistory":stats_history(&enemy.stats_history)
        }))}).collect();
        json!({"lineup":lineup,"turnHistory":turns,"skillHistory":skills,"dataAvatar":[],
            "totalAV":self.action_value,"totalDamage":self.total_damage,"damagePerAV":dpav(self.total_damage,self.action_value),
            "cycleIndex":self.cycle,"waveIndex":self.wave,"maxWave":self.max_waves,"maxCycle":self.max_cycles,
            "version":version,"avatarDetail":avatars,"enemyDetail":enemies,
            "_nous":{"partial":self.partial,"interrupted":self.interrupted,
                "observedStatsHistory":true,"entityTurnOffset":self.entity_turn_offset}})
    }

    /// The source's 24-column comprehensive CSV with summaries followed by skills.
    pub fn export_csv(&self) -> String {
        let mut output = String::from(
            "data_type,character_name,character_id,total_damage,damage_percentage,dpav,primary_skill_usage_count,turns_taken,average_damage_per_turn,max_single_turn_damage,first_turn_number,last_turn_number,turn_order,turn_battle_id,wave,cycle,action_value,skill_name,skill_type,skill_type_name,skill_damage,cumulative_damage,cumulative_character_damage,skill_damage_percentage\n",
        );
        for (avatar_index, avatar) in self.avatars.iter().enumerate() {
            let turns: Vec<_> = self
                .turn_history
                .iter()
                .enumerate()
                .filter_map(|(index, turn)| {
                    let damage = turn
                        .avatars_turn_damage
                        .get(avatar_index)
                        .copied()
                        .unwrap_or(0.0);
                    (damage > 0.0).then_some((index + 1, damage))
                })
                .collect();
            let mut skill_usage = BTreeMap::<&str, usize>::new();
            for skill in self
                .skills
                .iter()
                .filter(|skill| skill.avatar_id == avatar.id)
            {
                *skill_usage.entry(&skill.name).or_default() += 1;
            }
            let usage = skill_usage.values().copied().max().unwrap_or(0);
            let mut row = vec![String::new(); 24];
            row[0] = "character_summary".into();
            row[1] = avatar.name.clone();
            row[2] = avatar.id.to_string();
            row[3] = number(avatar.damage);
            row[4] = number(percentage(avatar.damage, self.total_damage));
            row[5] = number(dpav(avatar.damage, self.action_value));
            row[6] = usage.to_string();
            row[7] = turns.len().to_string();
            row[8] = number(if turns.is_empty() {
                0.0
            } else {
                avatar.damage / turns.len() as f64
            });
            row[9] = number(turns.iter().map(|(_, damage)| *damage).fold(0.0, f64::max));
            row[10] = turns
                .first()
                .map(|(index, _)| *index)
                .unwrap_or(0)
                .to_string();
            row[11] = turns
                .last()
                .map(|(index, _)| *index)
                .unwrap_or(0)
                .to_string();
            write_row(&mut output, &row);
        }
        let mut total = 0.0;
        let mut cumulative = BTreeMap::<u32, f64>::new();
        for (index, skill) in self.skills.iter().enumerate() {
            total += skill.damage;
            let character_total = cumulative.entry(skill.avatar_id).or_default();
            *character_total += skill.damage;
            // turnBattleId is one-based in the source JSON; correct its CSV lookup.
            let turn: Option<&EntityTurn> = skill
                .turn_index
                .checked_sub(self.entity_turn_offset + 1)
                .and_then(|index| self.entity_turns.get(index));
            let mut row = vec![String::new(); 24];
            row[0] = "skill_detail".into();
            row[1] = self
                .avatars
                .iter()
                .find(|avatar| avatar.id == skill.avatar_id)
                .map(|avatar| avatar.name.clone())
                .unwrap_or_else(|| format!("Avatar_{}", skill.avatar_id));
            row[2] = skill.avatar_id.to_string();
            row[12] = (index + 1).to_string();
            row[13] = skill.turn_index.to_string();
            row[14] = turn.map(|turn| turn.wave).unwrap_or(1).to_string();
            row[15] = turn.map(|turn| turn.cycle).unwrap_or(1).to_string();
            row[16] = number(turn.map(|turn| turn.action_value).unwrap_or(0.0));
            row[17] = skill.name.clone();
            row[18] = skill.skill_type.clone();
            row[19] = skill.skill_type.clone();
            row[20] = number(skill.damage);
            row[21] = number(total);
            row[22] = number(*character_total);
            row[23] = number(percentage(skill.damage, self.total_damage));
            write_row(&mut output, &row);
        }
        output
    }
}

fn percentage(value: f64, total: f64) -> f64 {
    if total > 0.0 {
        value / total * 100.0
    } else {
        0.0
    }
}
fn number(value: f64) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "0.0".into())
}

fn tracked_stats(properties: &BTreeMap<String, f64>) -> BTreeMap<&str, f64> {
    [
        "CurrentHP",
        "MaxHP",
        "Attack",
        "Defence",
        "Speed",
        "ActionDelay",
    ]
    .into_iter()
    .filter_map(|key| properties.get(key).map(|value| (key, *value)))
    .collect()
}

fn stats_history(observations: &[StatsObservation]) -> Value {
    let history: Vec<_> = observations
        .iter()
        .filter_map(|observation| {
            let stats = tracked_stats(&observation.properties);
            (!stats.is_empty())
                .then(|| json!({"stats":stats,"turnBattleId":observation.turn_index}))
        })
        .collect();
    json!(history)
}

fn write_row(output: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        // A spreadsheet runs a text cell that starts like a formula. Names come
        // from the local event stream, which anything on this PC can write to:
        // such text gets a leading apostrophe. Numbers ("-5.5") stay numbers.
        let formula = value.starts_with(['=', '+', '-', '@', '\t', '\r'])
            && value.parse::<f64>().is_err();
        let value = if formula {
            std::borrow::Cow::Owned(format!("'{value}"))
        } else {
            std::borrow::Cow::Borrowed(value.as_str())
        };
        if value.contains([',', '"', '\n', '\r']) {
            output.push('"');
            output.push_str(&value.replace('"', "\"\""));
            output.push('"');
        } else {
            output.push_str(&value);
        }
    }
    output.push('\n');
}

#[cfg(test)]
mod csv_tests {
    use super::write_row;

    #[test]
    fn text_that_looks_like_a_formula_is_not_run_by_a_spreadsheet() {
        let mut out = String::new();
        let row = ["=1+1", "@SUM(A1)", "+cmd|' /C calc'!A0", "-5.5", "-2+3", "Ахерон", "a,b"];
        write_row(&mut out, &row.map(String::from));
        assert_eq!(out, "'=1+1,'@SUM(A1),'+cmd|' /C calc'!A0,-5.5,'-2+3,Ахерон,\"a,b\"\n");
    }
}
