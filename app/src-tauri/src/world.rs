//! Where the player is and what's done in the world: location, position,
//! opened chests (per floor) and the state of every object seen (PropState:
//! anchors, puzzles, chests). Protocol notes: docs "Протокол — карта и сундуки".

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use reliquary::network::command::GameCommand;
use reliquary::network::command::command_id as cmd;
use reliquary::network::command::proto;
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
#[serde(rename_all = "camelCase")]
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

impl World {
    pub fn handle(&mut self, command: &GameCommand) -> WorldChange {
        match command.command_id {
            cmd::EnterSceneByServerScNotify => {
                let Ok(n) = command.parse_proto::<proto::EnterSceneByServerScNotify::EnterSceneByServerScNotify>() else {
                    return WorldChange::None;
                };
                n.scene.as_ref().map(|s| self.apply_scene(s)).map_or(WorldChange::None, |_| WorldChange::Changed)
            }
            cmd::GetCurSceneInfoScRsp => {
                let Ok(r) = command.parse_proto::<proto::GetCurSceneInfoScRsp::GetCurSceneInfoScRsp>() else {
                    return WorldChange::None;
                };
                r.scene.as_ref().map(|s| self.apply_scene(s)).map_or(WorldChange::None, |_| WorldChange::Changed)
            }
            cmd::SceneEntityMoveCsReq => {
                let Ok(req) = command.parse_proto::<proto::SceneEntityMoveCsReq::SceneEntityMoveCsReq>() else {
                    return WorldChange::None;
                };
                // the controlled avatar is the scene leader; fall back to the first mover
                let entity = req
                    .entity_motion_list
                    .iter()
                    .find(|m| Some(m.entity_id) == self.leader_entity_id)
                    .or(req.entity_motion_list.first());
                let Some(pos) = entity.and_then(|m| Some(Position { layer: Some(m.map_layer), ..position(m.motion.as_ref()?)? }))
                else {
                    return WorldChange::None;
                };
                self.player = Some(pos);
                if self.last_move_emit.is_none_or(|t| t.elapsed() >= MOVE_EMIT_INTERVAL) {
                    self.last_move_emit = Some(Instant::now());
                    WorldChange::Moved
                } else {
                    WorldChange::None
                }
            }
            cmd::InteractPropScRsp => {
                let Ok(r) = command.parse_proto::<proto::InteractPropScRsp::InteractPropScRsp>() else {
                    return WorldChange::None;
                };
                match (self.prop_entities.get(&r.prop_entity_id).copied(), self.floor_id) {
                    (Some((group, config)), Some(floor)) => {
                        self.set_prop(floor, group, config, r.prop_state);
                        WorldChange::Changed
                    }
                    _ => WorldChange::None,
                }
            }
            cmd::SceneGroupRefreshScNotify => {
                let Ok(n) = command.parse_proto::<proto::SceneGroupRefreshScNotify::SceneGroupRefreshScNotify>() else {
                    return WorldChange::None;
                };
                use proto::SceneEntityRefreshInfo::scene_entity_refresh_info::Refresh_type;
                let floor = if n.floor_id != 0 { n.floor_id } else { self.floor_id.unwrap_or_default() };
                for group in &n.group_refresh_list {
                    for r in &group.refresh_entity {
                        if let Some(Refresh_type::AddEntity(e)) = &r.refresh_type {
                            self.track_entity(floor, e);
                        }
                    }
                }
                WorldChange::Changed
            }
            cmd::GetSceneMapInfoScRsp => {
                let Ok(r) = command.parse_proto::<proto::GetSceneMapInfoScRsp::GetSceneMapInfoScRsp>() else {
                    return WorldChange::None;
                };
                for info in &r.scene_map_info {
                    // field names after 2.6 are scrambled: the opened MazeChest ids arrive in
                    // `lighten_section_list` (verified on game 4.6)
                    self.opened_chests.insert(info.floor_id, info.lighten_section_list.iter().copied().collect());
                    for p in &info.maze_prop_list {
                        self.set_prop(info.floor_id, p.group_id, p.config_id, p.state);
                    }
                    self.opened_known.insert(info.floor_id);
                }
                WorldChange::Changed
            }
            cmd::OpenChestScNotify => {
                let Ok(n) = command.parse_proto::<proto::OpenChestScNotify::OpenChestScNotify>() else {
                    return WorldChange::None;
                };
                let Some(floor) = self.floor_id else { return WorldChange::None };
                self.opened_chests.entry(floor).or_default().insert(n.chest_id);
                if let Some(at) = self.player {
                    self.opened_at.entry(floor).or_default().insert(n.chest_id, at);
                }
                WorldChange::Changed
            }
            _ => WorldChange::None,
        }
    }

    fn apply_scene(&mut self, scene: &proto::SceneInfo::SceneInfo) {
        self.plane_id = Some(scene.plane_id);
        self.floor_id = Some(scene.floor_id);
        self.entry_id = Some(scene.entry_id);
        self.leader_entity_id = Some(scene.leader_entity_id);
        // keep the layer we knew until the first movement tells otherwise
        let layer = self.player.and_then(|p| p.layer);
        self.player = scene
            .entity_list
            .iter()
            .find(|e| e.entity_id == scene.leader_entity_id)
            .and_then(|e| e.motion.as_ref())
            .and_then(position)
            .map(|p| Position { layer, ..p });

        self.prop_entities.clear();
        let entities = scene.entity_list.iter().chain(scene.entity_group_list.iter().flat_map(|g| g.entity_list.iter()));
        for e in entities {
            self.track_entity(scene.floor_id, e);
        }
    }

    fn track_entity(&mut self, floor: u32, e: &proto::SceneEntityInfo::SceneEntityInfo) {
        use proto::SceneEntityInfo::scene_entity_info::Entity;
        if let Some(Entity::Prop(prop)) = &e.entity {
            self.prop_entities.insert(e.entity_id, (e.group_id, e.inst_id));
            self.set_prop(floor, e.group_id, e.inst_id, prop.prop_state);
        }
    }

    fn set_prop(&mut self, floor: u32, group: u32, config: u32, state: u32) {
        self.props.entry(floor).or_default().insert(format!("{group}:{config}"), state);
    }
}

fn position(motion: &proto::MotionInfo::MotionInfo) -> Option<Position> {
    let pos = motion.pos_index.as_ref()?;
    Some(Position {
        x: pos.x,
        y: pos.y,
        z: pos.z,
        rot_y: motion.rot_index.as_ref().map(|r| r.y).unwrap_or_default(),
        layer: None,
    })
}
