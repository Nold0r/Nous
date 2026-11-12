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
    /// A move arrived inside the emit interval and was not announced yet.
    #[serde(skip)]
    move_pending: bool,
    /// Floors whose stored data changed since `take_touched`.
    #[serde(skip)]
    touched: BTreeSet<u32>,
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
                // the map layer belongs to the floor: another floor starts without one
                let same_floor = self.floor_id == Some(*floor);
                let layer = self.player.and_then(|p| p.layer).filter(|_| same_floor);
                self.plane_id = Some(*plane);
                self.floor_id = Some(*floor);
                self.entry_id = Some(*entry);
                self.leader_entity_id = Some(*leader);
                self.player = entities
                    .iter()
                    .find(|e| e.id == *leader)
                    .and_then(|e| e.position)
                    .map(|p| Position { layer, ..p });
                self.prop_entities.clear();
                for e in entities {
                    self.track_entity(*floor, e);
                }
                self.touched.insert(*floor);
                WorldChange::Changed
            }
            WorldCommand::Move(entities) => {
                // Until a scene names the leader (Nous started mid-session) the
                // first mover is taken; once it is known, others are not the player.
                let mover = match self.leader_entity_id {
                    Some(leader) => entities.iter().find(|(id, _)| *id == leader),
                    None => entities.first(),
                };
                let Some((_, pos)) = mover else {
                    return WorldChange::None;
                };
                self.player = Some(*pos);
                if self
                    .last_move_emit
                    .is_none_or(|t| t.elapsed() >= MOVE_EMIT_INTERVAL)
                {
                    self.last_move_emit = Some(Instant::now());
                    self.move_pending = false;
                    WorldChange::Moved
                } else {
                    self.move_pending = true;
                    WorldChange::None
                }
            }
            WorldCommand::Interact { entity, state } => {
                match (self.prop_entities.get(entity).copied(), self.floor_id) {
                    (Some((group, config)), Some(floor)) => {
                        changed(self.set_prop(floor, group, config, *state))
                    }
                    _ => WorldChange::None,
                }
            }
            // The game repeats these as scene groups load around the player:
            // only a prop whose state really differs is a change (an event, a
            // new revision and a rewrite of the account file).
            WorldCommand::Refresh { floor, entities } => {
                let floor = if *floor != 0 {
                    *floor
                } else {
                    self.floor_id.unwrap_or_default()
                };
                let mut any = false;
                for e in entities {
                    any |= self.track_entity(floor, e);
                }
                changed(any)
            }
            WorldCommand::Map(floors) => {
                let mut any = false;
                for info in floors {
                    if self.opened_chests.get(&info.floor) != Some(&info.chests) {
                        self.opened_chests.insert(info.floor, info.chests.clone());
                        self.touched.insert(info.floor);
                        any = true;
                    }
                    for (group, config, state) in &info.props {
                        any |= self.set_prop(info.floor, *group, *config, *state);
                    }
                    any |= self.opened_known.insert(info.floor);
                }
                changed(any)
            }
            WorldCommand::OpenChest(id) => {
                let Some(floor) = self.floor_id else {
                    return WorldChange::None;
                };
                if !self.opened_chests.entry(floor).or_default().insert(*id) {
                    return WorldChange::None;
                }
                if let Some(at) = self.player {
                    self.opened_at.entry(floor).or_default().insert(*id, at);
                }
                self.touched.insert(floor);
                WorldChange::Changed
            }
        }
    }
    /// Payload of `world-changed`: where the player is and the full data of
    /// the given floors only. The whole world is tens of thousands of props;
    /// a reader merges these floors into the world it read before.
    pub fn patch(&self, floors: &BTreeSet<u32>) -> serde_json::Value {
        let floors: serde_json::Map<String, serde_json::Value> = floors
            .iter()
            .map(|floor| {
                (
                    floor.to_string(),
                    serde_json::json!({
                        "openedChests": self.opened_chests.get(floor).cloned().unwrap_or_default(),
                        "openedAt": self.opened_at.get(floor).cloned().unwrap_or_default(),
                        "props": self.props.get(floor).cloned().unwrap_or_default(),
                    }),
                )
            })
            .collect();
        serde_json::json!({
            "planeId": self.plane_id,
            "floorId": self.floor_id,
            "entryId": self.entry_id,
            "player": self.player,
            "floors": floors,
        })
    }
    /// Floors whose stored data changed since the last call.
    pub fn take_touched(&mut self) -> BTreeSet<u32> {
        std::mem::take(&mut self.touched)
    }
    /// The position of a move that was not announced because it came inside
    /// the emit interval, once that interval has passed. Without this the
    /// place where the player stopped never reached the map.
    pub fn take_pending_move(&mut self) -> Option<Position> {
        let due = self
            .last_move_emit
            .is_none_or(|t| t.elapsed() >= MOVE_EMIT_INTERVAL);
        if !(self.move_pending && due) {
            return None;
        }
        self.move_pending = false;
        self.last_move_emit = Some(Instant::now());
        self.player
    }
    /// True if a prop state was stored or changed.
    fn track_entity(&mut self, floor: u32, e: &Entity) -> bool {
        let Some(state) = e.prop_state else {
            return false;
        };
        self.prop_entities.insert(e.id, (e.group, e.config));
        self.set_prop(floor, e.group, e.config, state)
    }
    /// True if the state differs from the stored one.
    fn set_prop(&mut self, floor: u32, group: u32, config: u32, state: u32) -> bool {
        let previous = self
            .props
            .entry(floor)
            .or_default()
            .insert(format!("{group}:{config}"), state);
        let differs = previous != Some(state);
        if differs {
            self.touched.insert(floor);
        }
        differs
    }
}
fn changed(any: bool) -> WorldChange {
    if any {
        WorldChange::Changed
    } else {
        WorldChange::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(id: u32, state: u32) -> Entity {
        Entity { id, group: 5, config: id, prop_state: Some(state), position: None }
    }
    fn at(x: i32) -> Position {
        Position { x, y: 0, z: 0, rot_y: 0, layer: Some(0) }
    }
    fn scene(floor: u32, entities: Vec<Entity>) -> WorldCommand {
        WorldCommand::Scene { plane: 1, floor, entry: 1, leader: 100, entities }
    }

    #[test]
    fn a_refresh_that_brings_nothing_new_is_not_a_change() {
        let mut world = World::default();
        world.handle(&scene(10, vec![prop(1, 12)]));
        world.take_touched();

        let empty = WorldCommand::Refresh { floor: 10, entities: vec![] };
        assert_eq!(world.handle(&empty), WorldChange::None, "empty refresh");
        let same = WorldCommand::Refresh { floor: 10, entities: vec![prop(1, 12)] };
        assert_eq!(world.handle(&same), WorldChange::None, "the same state again");
        assert!(world.take_touched().is_empty());

        let opened = WorldCommand::Refresh { floor: 10, entities: vec![prop(1, 13)] };
        assert_eq!(world.handle(&opened), WorldChange::Changed);
        assert_eq!(world.take_touched(), BTreeSet::from([10]));
        assert_eq!(world.props[&10]["5:1"], 13);
    }

    #[test]
    fn repeated_map_chest_and_interaction_answers_are_not_changes() {
        let mut world = World::default();
        world.handle(&scene(10, vec![prop(1, 12)]));
        let map = WorldCommand::Map(vec![FloorMap {
            floor: 20,
            chests: BTreeSet::from([7]),
            props: vec![(5, 2, 8)],
        }]);
        assert_eq!(world.handle(&map), WorldChange::Changed);
        world.take_touched();
        assert_eq!(world.handle(&map), WorldChange::None, "the same map again");

        assert_eq!(world.handle(&WorldCommand::OpenChest(3)), WorldChange::Changed);
        assert_eq!(world.take_touched(), BTreeSet::from([10]));
        assert_eq!(world.handle(&WorldCommand::OpenChest(3)), WorldChange::None);

        let interact = WorldCommand::Interact { entity: 1, state: 13 };
        assert_eq!(world.handle(&interact), WorldChange::Changed);
        assert_eq!(world.handle(&interact), WorldChange::None);
    }

    #[test]
    fn another_floor_does_not_inherit_the_layer_and_strangers_do_not_move_the_player() {
        let leader = |layer| Entity { id: 100, group: 0, config: 0, prop_state: None, position: Some(Position { layer, ..at(5) }) };
        let mut world = World::default();
        world.handle(&scene(10, vec![leader(None)]));
        world.handle(&WorldCommand::Move(vec![(100, Position { layer: Some(3), ..at(6) })]));
        world.handle(&scene(10, vec![leader(None)]));
        assert_eq!(world.player.unwrap().layer, Some(3), "the same floor keeps the layer");
        world.handle(&scene(20, vec![leader(None)]));
        assert_eq!(world.player.unwrap().layer, None, "another floor starts without it");

        let before = world.player;
        assert_eq!(world.handle(&WorldCommand::Move(vec![(777, at(999))])), WorldChange::None);
        assert_eq!(world.player, before, "an entity that is not the leader is not the player");
    }

    #[test]
    fn the_position_after_the_last_move_is_kept_for_a_late_delivery() {
        let mut world = World::default();
        world.handle(&scene(10, vec![]));
        assert_eq!(world.handle(&WorldCommand::Move(vec![(100, at(1))])), WorldChange::Moved);
        assert_eq!(world.take_pending_move(), None, "delivered at once, nothing is pending");

        // too soon after the first one: not announced
        assert_eq!(world.handle(&WorldCommand::Move(vec![(100, at(2))])), WorldChange::None);
        assert_eq!(world.take_pending_move(), None, "still inside the interval");
        std::thread::sleep(MOVE_EMIT_INTERVAL + Duration::from_millis(20));
        assert_eq!(world.take_pending_move(), Some(at(2)));
        assert_eq!(world.take_pending_move(), None, "delivered once");
    }
}
