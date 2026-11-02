//! Portable, game-independent Veritas battle state.

mod export;
mod models;
#[cfg(test)]
mod tests;
pub mod wire;
pub use models::*;
use std::time::{SystemTime, UNIX_EPOCH};

const HISTORY_LIMIT: usize = 20;
const DETAIL_LIMIT: usize = 4096;
const ENTITY_LIMIT: usize = 256;
const PROPERTY_LIMIT: usize = 256;

#[derive(Default)]
pub struct Engine {
    history: Vec<Battle>,
    current: Option<Battle>,
    connected: bool,
    revision: u64,
    next_id: u64,
    began: bool,
    lineup: bool,
    diagnostics: Vec<String>,
}

impl Engine {
    /// Decode completely before changing state, so malformed payloads are atomic.
    pub fn apply(&mut self, name: &str, data: serde_json::Value) -> Result<bool, String> {
        let Some(event) = wire::decode(name, data)? else {
            return Ok(false);
        };
        use wire::Event::*;
        match event {
            Connected { version } => {
                self.connected = true;
                self.diagnostic(format!(
                    "Veritas {version}; relative AV reset is not exposed by this protocol"
                ));
            }
            Error { msg } => self.diagnostic(msg),
            OnBattleBegin {
                max_waves,
                max_cycles,
                stage_id,
            } => {
                if self.active() && self.began {
                    self.finish_interrupted();
                }
                self.ensure_battle(false);
                self.began = true;
                let battle = self.current.as_mut().unwrap();
                battle.max_waves = max_waves;
                battle.max_cycles = max_cycles;
                battle.stage_id = stage_id;
                if self.lineup && battle.total_damage == 0.0 && battle.turn_history.is_empty() {
                    battle.partial = false;
                }
            }
            OnSetBattleLineup { avatars } => {
                if self.active() && self.lineup {
                    self.finish_interrupted();
                }
                self.ensure_battle(false);
                self.lineup = true;
                let battle = self.current.as_mut().unwrap();
                merge_avatars(battle, avatars);
                if self.began && battle.total_damage == 0.0 && battle.turn_history.is_empty() {
                    battle.partial = false;
                }
            }
            OnBattleEnd {
                avatars,
                turn_history,
                av_history: _,
                turn_count,
                total_damage,
                action_value,
                cycle,
                wave,
                stage_id,
            } => {
                // Repeated final summaries must not create a second archived battle.
                if self
                    .current
                    .as_ref()
                    .is_some_and(|battle| battle.ended_at.is_some())
                {
                    return Ok(false);
                }
                self.ensure_battle(true);
                let battle = self.current.as_mut().unwrap();
                if (battle.total_damage - total_damage).abs() > 0.001
                    || turn_count > battle.turn_history.len()
                {
                    battle.partial = true;
                }
                merge_avatars(battle, avatars);
                battle.turn_history = turn_history.into_iter().map(Into::into).collect();
                if battle.turn_history.len() > DETAIL_LIMIT {
                    battle.partial = true;
                }
                retain_tail(&mut battle.turn_history, DETAIL_LIMIT);
                battle.av_history = aggregate_av(&battle.turn_history);
                // Final history repairs missed turn packets, but cannot repair lost type/overkill/skill data.
                for (index, avatar) in battle.avatars.iter_mut().enumerate() {
                    let recovered = battle
                        .turn_history
                        .iter()
                        .map(|turn| turn.avatars_turn_damage.get(index).copied().unwrap_or(0.0))
                        .sum::<f64>();
                    avatar.damage = avatar.damage.max(recovered);
                }
                battle.total_damage = total_damage;
                battle.action_value = action_value;
                battle.cycle = cycle;
                battle.wave = wave;
                if stage_id != 0 {
                    battle.stage_id = stage_id;
                }
                bound_battle(battle);
                self.finish(false);
            }
            event => {
                if self
                    .current
                    .as_ref()
                    .is_some_and(|battle| battle.ended_at.is_some())
                {
                    return Ok(false);
                }
                self.ensure_battle(true);
                let battle = self.current.as_mut().unwrap();
                match event {
                    OnDamage {
                        attacker,
                        damage,
                        overkill_damage,
                        r#type,
                    } => {
                        if attacker.team != Team::Player {
                            return Ok(false);
                        }
                        let avatar = avatar_mut(battle, attacker.uid);
                        avatar.damage += damage;
                        avatar.overkill += overkill_damage;
                        if avatar.by_type.len() < PROPERTY_LIMIT
                            || avatar.by_type.contains_key(&r#type)
                        {
                            *avatar.by_type.entry(r#type.clone()).or_default() += damage;
                        } else {
                            battle.partial = true;
                        }
                        battle.total_damage += damage;
                        if let Some(skill) = battle
                            .skills
                            .iter_mut()
                            .rev()
                            .find(|skill| skill.avatar_id == attacker.uid)
                        {
                            skill.damage += damage;
                            if skill.details.len() < DETAIL_LIMIT {
                                skill.details.push(DamageDetail {
                                    damage,
                                    overkill_damage,
                                    r#type,
                                });
                            } else {
                                battle.partial = true;
                            }
                        }
                    }
                    OnTurnBegin {
                        action_value,
                        turn_owner,
                    } => {
                        battle.action_value = action_value;
                        if let Some(owner) = turn_owner {
                            battle.entity_turns.push(EntityTurn {
                                owner,
                                action_value,
                                wave: battle.wave,
                                cycle: battle.cycle,
                            });
                        }
                    }
                    OnTurnEnd { turn_info } => {
                        let turn: TurnInfo = turn_info.into();
                        if turn.avatars_turn_damage.len() != battle.avatars.len() {
                            battle.partial = true;
                        }
                        battle.action_value = turn.action_value;
                        battle.wave = turn.wave;
                        battle.cycle = turn.cycle;
                        for (index, avatar) in battle.avatars.iter_mut().enumerate() {
                            let recovered = battle
                                .turn_history
                                .iter()
                                .map(|old| {
                                    old.avatars_turn_damage.get(index).copied().unwrap_or(0.0)
                                })
                                .sum::<f64>()
                                + turn.avatars_turn_damage.get(index).copied().unwrap_or(0.0);
                            if recovered > avatar.damage + 0.001 {
                                battle.partial = true;
                                battle.total_damage += recovered - avatar.damage;
                                avatar.damage = recovered;
                            }
                        }
                        merge_av(&mut battle.av_history, &turn);
                        battle.turn_history.push(turn);
                    }
                    OnUseSkill { avatar, skill } => {
                        battle.skills.push(SkillRecord {
                            avatar_id: avatar.uid,
                            name: skill.name,
                            skill_type: skill.r#type,
                            skill_config_id: skill.skill_config_id,
                            damage: 0.0,
                            details: Vec::new(),
                            turn_index: battle.entity_turns.len(),
                        });
                    }
                    OnUpdateWave { wave } => battle.wave = wave,
                    OnUpdateCycle { cycle } => battle.cycle = cycle,
                    OnStatChange { entity, property } => {
                        let properties = match entity.team {
                            Team::Player => &mut avatar_mut(battle, entity.uid).properties,
                            Team::Enemy => &mut enemy_mut(battle, entity.uid).properties,
                        };
                        if properties.len() < PROPERTY_LIMIT
                            || properties.contains_key(&property.r#type)
                        {
                            properties.insert(property.r#type, property.value);
                        } else {
                            battle.partial = true;
                        }
                    }
                    OnEntityDefeated {
                        entity_defeated, ..
                    } => match entity_defeated.team {
                        Team::Player => avatar_mut(battle, entity_defeated.uid).dead = true,
                        Team::Enemy => enemy_mut(battle, entity_defeated.uid).dead = true,
                    },
                    OnInitializeEnemy { enemy } => {
                        let target = enemy_mut(battle, enemy.uid);
                        target.id = enemy.id;
                        target.name = enemy.name;
                        for (key, value) in
                            enemy.base_stats.properties.into_iter().take(PROPERTY_LIMIT)
                        {
                            target.properties.entry(key).or_insert(value);
                        }
                    }
                    OnUpdateTeamFormation { entities, team } => {
                        if team == Team::Enemy {
                            // Keep inactive enemy data for export, place the active formation first.
                            let mut ordered = Vec::new();
                            for entity in entities.into_iter().take(ENTITY_LIMIT) {
                                if let Some(index) = battle
                                    .enemies
                                    .iter()
                                    .position(|enemy| enemy.uid == entity.uid)
                                {
                                    ordered.push(battle.enemies.remove(index));
                                }
                            }
                            ordered.append(&mut battle.enemies);
                            battle.enemies = ordered;
                        }
                    }
                    _ => unreachable!(),
                }
                bound_battle(battle);
            }
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }

    pub fn set_connected(&mut self, connected: bool) {
        if !connected {
            self.finish_interrupted();
        }
        if self.connected != connected {
            self.connected = connected;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn finish_interrupted(&mut self) {
        self.finish(true);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            revision: self.revision,
            connected: self.connected,
            active: self.active(),
            battle: self.current.clone(),
            history: self.history.iter().map(BattleSummary::from).collect(),
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn history(&self) -> &[Battle] {
        &self.history
    }

    pub fn restore_history(&mut self, mut history: Vec<Battle>) {
        history.sort_by_key(|battle| battle.id);
        history.dedup_by_key(|battle| battle.id);
        retain_tail(&mut history, HISTORY_LIMIT);
        for battle in &mut history {
            if battle.ended_at.is_none() {
                battle.ended_at = Some(now_millis());
                battle.interrupted = true;
                battle.partial = true;
            }
            bound_battle(battle);
        }
        self.next_id = self
            .next_id
            .max(history.iter().map(|battle| battle.id).max().unwrap_or(0));
        if !self.active() {
            self.current = history.last().cloned();
        }
        self.history = history;
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn battle(&self, id: u64) -> Option<Battle> {
        self.current
            .iter()
            .chain(self.history.iter())
            .find(|battle| battle.id == id)
            .cloned()
    }

    fn active(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|battle| battle.ended_at.is_none())
    }

    fn ensure_battle(&mut self, partial: bool) {
        if self.active() {
            return;
        }
        self.next_id = self.next_id.saturating_add(1).max(now_millis() as u64);
        self.current = Some(Battle {
            id: self.next_id,
            started_at: now_millis(),
            partial,
            ..Battle::default()
        });
        self.began = false;
        self.lineup = false;
    }

    fn finish(&mut self, interrupted: bool) {
        if !self.active() {
            return;
        }
        let battle = self.current.as_mut().unwrap();
        battle.ended_at = Some(now_millis());
        battle.interrupted = interrupted;
        if !self.began || !self.lineup {
            battle.partial = true;
        }
        self.history.push(battle.clone());
        retain_tail(&mut self.history, HISTORY_LIMIT);
        self.revision = self.revision.wrapping_add(1);
    }

    fn diagnostic(&mut self, message: String) {
        self.diagnostics.push(message.chars().take(2048).collect());
        retain_tail(&mut self.diagnostics, 128);
    }
}

pub fn visible(
    enabled: bool,
    connected: bool,
    active: bool,
    target_visible: bool,
    manual_hidden: bool,
    exclusive: bool,
) -> bool {
    enabled && connected && active && target_visible && !manual_hidden && !exclusive
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

impl From<&Battle> for BattleSummary {
    fn from(battle: &Battle) -> Self {
        Self {
            id: battle.id,
            stage_id: battle.stage_id,
            total_damage: battle.total_damage,
            action_value: battle.action_value,
            ended_at: battle.ended_at,
            interrupted: battle.interrupted,
            partial: battle.partial,
        }
    }
}

fn avatar_mut(battle: &mut Battle, id: u32) -> &mut Combatant {
    let index = match battle.avatars.iter().position(|avatar| avatar.id == id) {
        Some(index) => index,
        None => {
            battle.partial = true;
            battle.avatars.push(Combatant {
                id,
                name: format!("Avatar_{id}"),
                ..Combatant::default()
            });
            battle.avatars.len() - 1
        }
    };
    &mut battle.avatars[index]
}

fn enemy_mut(battle: &mut Battle, uid: u32) -> &mut EnemyState {
    let index = match battle.enemies.iter().position(|enemy| enemy.uid == uid) {
        Some(index) => index,
        None => {
            battle.enemies.push(EnemyState {
                uid,
                name: format!("Enemy_{uid}"),
                ..EnemyState::default()
            });
            battle.enemies.len() - 1
        }
    };
    &mut battle.enemies[index]
}

fn merge_avatars(battle: &mut Battle, avatars: Vec<wire::Avatar>) {
    if avatars.len() > ENTITY_LIMIT {
        battle.partial = true;
    }
    let mut ordered = Vec::new();
    for avatar in avatars.into_iter().take(ENTITY_LIMIT) {
        let mut combatant = battle
            .avatars
            .iter()
            .find(|current| current.id == avatar.id)
            .cloned()
            .unwrap_or_default();
        combatant.id = avatar.id;
        combatant.name = avatar.name;
        ordered.push(combatant);
    }
    battle.avatars = ordered;
}

fn merge_av(history: &mut Vec<TurnInfo>, turn: &TurnInfo) {
    if let Some(previous) = history
        .last_mut()
        .filter(|previous| previous.action_value == turn.action_value)
    {
        previous.avatars_turn_damage.resize(
            previous
                .avatars_turn_damage
                .len()
                .max(turn.avatars_turn_damage.len()),
            0.0,
        );
        for (index, damage) in turn.avatars_turn_damage.iter().enumerate() {
            previous.avatars_turn_damage[index] += damage;
        }
        previous.total_damage += turn.total_damage;
    } else {
        history.push(turn.clone());
    }
}

fn aggregate_av(turns: &[TurnInfo]) -> Vec<TurnInfo> {
    let mut history = Vec::new();
    for turn in turns {
        merge_av(&mut history, turn);
    }
    history
}

fn retain_tail<T>(values: &mut Vec<T>, limit: usize) {
    if values.len() > limit {
        values.drain(..values.len() - limit);
    }
}

fn bound_battle(battle: &mut Battle) {
    if battle.avatars.len() > ENTITY_LIMIT
        || battle.enemies.len() > ENTITY_LIMIT
        || battle.turn_history.len() > DETAIL_LIMIT
        || battle.av_history.len() > DETAIL_LIMIT
        || battle.skills.len() > DETAIL_LIMIT
        || battle.entity_turns.len() > DETAIL_LIMIT
    {
        battle.partial = true;
    }
    battle.avatars.truncate(ENTITY_LIMIT);
    battle.enemies.truncate(ENTITY_LIMIT);
    retain_tail(&mut battle.turn_history, DETAIL_LIMIT);
    retain_tail(&mut battle.av_history, DETAIL_LIMIT);
    retain_tail(&mut battle.skills, DETAIL_LIMIT);
    retain_tail(&mut battle.entity_turns, DETAIL_LIMIT);
    for turn in battle
        .turn_history
        .iter_mut()
        .chain(battle.av_history.iter_mut())
    {
        if turn.avatars_turn_damage.len() > ENTITY_LIMIT {
            battle.partial = true;
        }
        turn.avatars_turn_damage.truncate(ENTITY_LIMIT);
    }
    for skill in &mut battle.skills {
        if skill.details.len() > DETAIL_LIMIT {
            battle.partial = true;
        }
        retain_tail(&mut skill.details, DETAIL_LIMIT);
    }
}
