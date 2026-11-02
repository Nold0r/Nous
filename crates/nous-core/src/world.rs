//! Where the player is and what's done in the world: location, position,
//! opened chests (per floor) and the state of every object seen (PropState:
//! anchors, puzzles, chests). Protocol notes: docs "Протокол — карта и сундуки".

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Position is sent many times a second; the UI doesn't need more than this.
const MOVE_EMIT_INTERVAL: Duration = Duration::from_millis(100);

/// Player position as sent by the game: 1/1000 m, heading in 1/1000 degree.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub rot_y: i32,
    /// Map layer of multi-level areas from the move packet (often 0).
    pub layer: Option<u32>,
}

/// Stored part of the world state (in the account file).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct World {
    pub plane_id: Option<u32>,
    pub floor_id: Option<u32>,
    pub entry_id: Option<u32>,
    pub player: Option<Position>,
    /// floor id -> opened MazeChest ids
    pub opened_chests: BTreeMap<u32, BTreeSet<u32>>,
    /// floor id -> chest id -> where the player stood when opening it (puzzle
    /// reward chests appear far from their datamine position)
    pub opened_at: BTreeMap<u32, BTreeMap<u32, Position>>,
    /// floor id -> "group:config" -> PropState (8 anchor on, 12/13 chest closed/used, …)
    pub props: BTreeMap<u32, BTreeMap<String, u32>>,

    /// floors whose full opened list came this session (in-game map opened)
    #[serde(skip)]
    pub opened_known: BTreeSet<u32>,
    #[serde(skip)]
    leader_entity_id: Option<u32>,
    /// scene entity id -> (group, config) of props in the current scene
    #[serde(skip)]
    prop_entities: HashMap<u32, (u32, u32)>,
    #[serde(skip)]
    last_move_emit: Option<Instant>,
}

/// What changed in the world after a command.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum WorldChange {
    #[default]
    None,
    /// Only the player moved (throttled).
    Moved,
    /// Location, chests or objects changed.
    Changed,
}

/// Normalized world data; no wire types escape the HSR adapter.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: u32,
    pub group: u32,
    pub config: u32,
    pub prop_state: Option<u32>,
    pub position: Option<Position>,
}
#[derive(Debug, Clone)]
pub enum WorldCommand {
    Scene {
        plane: u32,
        floor: u32,
        entry: u32,
        leader: u32,
        entities: Vec<Entity>,
    },
    Move(Vec<(u32, Position)>),
    Interact {
        entity: u32,
        state: u32,
    },
    Refresh {
        floor: u32,
        entities: Vec<Entity>,
    },
    Map(Vec<FloorMap>),
    OpenChest(u32),
}
#[derive(Debug, Clone)]
pub struct FloorMap {
    pub floor: u32,
    pub chests: BTreeSet<u32>,
    pub props: Vec<(u32, u32, u32)>,
}
impl World {
    pub fn handle(&mut self, command: &WorldCommand) -> WorldChange {
        match command {
            WorldCommand::Scene {
                plane,
                floor,
                entry,
                leader,
                entities,
            } => {
                self.plane_id = Some(*plane);
                self.floor_id = Some(*floor);
                self.entry_id = Some(*entry);
                self.leader_entity_id = Some(*leader);
                let layer = self.player.and_then(|p| p.layer);
                self.player = entities
                    .iter()
                    .find(|e| e.id == *leader)
                    .and_then(|e| e.position)
                    .map(|p| Position { layer, ..p });
                self.prop_entities.clear();
                for e in entities {
                    self.track_entity(*floor, e);
                }
                WorldChange::Changed
            }
            WorldCommand::Move(entities) => {
                let Some((_, pos)) = entities
                    .iter()
                    .find(|(id, _)| Some(*id) == self.leader_entity_id)
                    .or(entities.first())
                else {
                    return WorldChange::None;
                };
                self.player = Some(*pos);
                if self
                    .last_move_emit
                    .is_none_or(|t| t.elapsed() >= MOVE_EMIT_INTERVAL)
                {
                    self.last_move_emit = Some(Instant::now());
                    WorldChange::Moved
                } else {
                    WorldChange::None
                }
            }
            WorldCommand::Interact { entity, state } => {
                match (self.prop_entities.get(entity).copied(), self.floor_id) {
                    (Some((group, config)), Some(floor)) => {
                        self.set_prop(floor, group, config, *state);
                        WorldChange::Changed
                    }
                    _ => WorldChange::None,
                }
            }
            WorldCommand::Refresh { floor, entities } => {
                let floor = if *floor != 0 {
                    *floor
                } else {
                    self.floor_id.unwrap_or_default()
                };
                for e in entities {
                    self.track_entity(floor, e);
                }
                WorldChange::Changed
            }
            WorldCommand::Map(floors) => {
                for info in floors {
                    self.opened_chests.insert(info.floor, info.chests.clone());
                    for (group, config, state) in &info.props {
                        self.set_prop(info.floor, *group, *config, *state);
                    }
                    self.opened_known.insert(info.floor);
                }
                WorldChange::Changed
            }
            WorldCommand::OpenChest(id) => {
                let Some(floor) = self.floor_id else {
                    return WorldChange::None;
                };
                self.opened_chests.entry(floor).or_default().insert(*id);
                if let Some(at) = self.player {
                    self.opened_at.entry(floor).or_default().insert(*id, at);
                }
                WorldChange::Changed
            }
        }
    }
    fn track_entity(&mut self, floor: u32, e: &Entity) {
        if let Some(state) = e.prop_state {
            self.prop_entities.insert(e.id, (e.group, e.config));
            self.set_prop(floor, e.group, e.config, state);
        }
    }
    fn set_prop(&mut self, floor: u32, group: u32, config: u32, state: u32) {
        self.props
            .entry(floor)
            .or_default()
            .insert(format!("{group}:{config}"), state);
    }
}
