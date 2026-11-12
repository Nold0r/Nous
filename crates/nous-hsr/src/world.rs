use nous_core::world::*;
use reliquary::network::command::{GameCommand, command_id as cmd, proto};

pub fn decode(command: &GameCommand) -> Option<WorldCommand> {
    Some(match command.command_id {
        cmd::EnterSceneByServerScNotify => scene(
            command
                .parse_proto::<proto::EnterSceneByServerScNotify::EnterSceneByServerScNotify>()
                .ok()?
                .scene
                .as_ref()?,
        )?,
        cmd::GetCurSceneInfoScRsp => scene(
            rsp!(command, GetCurSceneInfoScRsp)?
                .scene
                .as_ref()?,
        )?,
        cmd::SceneEntityMoveCsReq => {
            let r = command
                .parse_proto::<proto::SceneEntityMoveCsReq::SceneEntityMoveCsReq>()
                .ok()?;
            WorldCommand::Move(
                r.entity_motion_list
                    .iter()
                    .filter_map(|m| {
                        Some((
                            m.entity_id,
                            Position {
                                layer: Some(m.map_layer),
                                ..position(m.motion.as_ref()?)?
                            },
                        ))
                    })
                    .collect(),
            )
        }
        cmd::InteractPropScRsp => {
            let r = rsp!(command, InteractPropScRsp)?;
            WorldCommand::Interact {
                entity: r.prop_entity_id,
                state: r.prop_state,
            }
        }
        cmd::SceneGroupRefreshScNotify => {
            use proto::SceneEntityRefreshInfo::scene_entity_refresh_info::Refresh_type;
            let r = command
                .parse_proto::<proto::SceneGroupRefreshScNotify::SceneGroupRefreshScNotify>()
                .ok()?;
            WorldCommand::Refresh {
                floor: r.floor_id,
                entities: r
                    .group_refresh_list
                    .iter()
                    .flat_map(|g| &g.refresh_entity)
                    .filter_map(|r| match &r.refresh_type {
                        Some(Refresh_type::AddEntity(e)) => Some(entity(e)),
                        _ => None,
                    })
                    .collect(),
            }
        }
        cmd::GetSceneMapInfoScRsp => {
            let r = rsp!(command, GetSceneMapInfoScRsp)?;
            WorldCommand::Map(
                r.scene_map_info
                    .iter()
                    .map(|i| FloorMap {
                        floor: i.floor_id,
                        // Verified against game 4.6: this scrambled field contains opened chest ids.
                        chests: i.lighten_section_list.iter().copied().collect(),
                        props: i
                            .maze_prop_list
                            .iter()
                            .map(|p| (p.group_id, p.config_id, p.state))
                            .collect(),
                    })
                    .collect(),
            )
        }
        cmd::OpenChestScNotify => WorldCommand::OpenChest(
            command
                .parse_proto::<proto::OpenChestScNotify::OpenChestScNotify>()
                .ok()?
                .chest_id,
        ),
        _ => return None,
    })
}
fn scene(s: &proto::SceneInfo::SceneInfo) -> Option<WorldCommand> {
    Some(WorldCommand::Scene {
        plane: s.plane_id,
        floor: s.floor_id,
        entry: s.entry_id,
        leader: s.leader_entity_id,
        entities: s
            .entity_list
            .iter()
            .chain(s.entity_group_list.iter().flat_map(|g| &g.entity_list))
            .map(entity)
            .collect(),
    })
}
fn entity(e: &proto::SceneEntityInfo::SceneEntityInfo) -> Entity {
    use proto::SceneEntityInfo::scene_entity_info::Entity as WireEntity;
    Entity {
        id: e.entity_id,
        group: e.group_id,
        config: e.inst_id,
        position: e.motion.as_ref().and_then(position),
        prop_state: match &e.entity {
            Some(WireEntity::Prop(p)) => Some(p.prop_state),
            _ => None,
        },
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
