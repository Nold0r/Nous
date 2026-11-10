//! Map scan for checking the map marks (`NOUS_MAP_SCAN=1`): what the game
//! says about each floor, filtered to the map's needs, in
//! `<app data>/map-scan/<floor>.json`. Each visit adds to the file, so running
//! through every location once gives the full picture. Compared with the
//! datamine marks by `app/scripts/map-scan-check.py`.
//!
//! Per floor: every scene object seen (props with their MazeProp id and state,
//! NPCs, monsters; positions in 1/1000 m) and what the in-game map sends
//! (chest counts per type, opened chests, object states).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use reliquary::network::command::{GameCommand, command_id as cmd, proto};
use serde::{Deserialize, Serialize};

pub const DIR: &str = "map-scan";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct FloorScan {
    floor: u32,
    plane: u32,
    /// "group:inst" -> what stands there
    objects: BTreeMap<String, Object>,
    /// last in-game map reply for this floor
    map: Option<MapReply>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Object {
    /// prop / npc / monster
    kind: String,
    /// MazeProp id, NPC id or monster id
    id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    event: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state: Option<u32>,
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct MapReply {
    /// [chest type, exist, opened] (field names in the protocol are scrambled:
    /// which number is which is checked against the game's map screen)
    chests: Vec<[u32; 3]>,
    /// opened chest ids (lighten_section_list, see world.rs)
    opened: Vec<u32>,
    opened_ids: Vec<u32>,
    /// "group:config" -> PropState
    props: BTreeMap<String, u32>,
    teleports: Vec<u32>,
    groups: Vec<u32>,
    saved: BTreeMap<String, i32>,
}

pub struct MapScan {
    dir: Option<PathBuf>,
    floors: BTreeMap<u32, FloorScan>,
    floor: Option<u32>,
}

impl MapScan {
    pub fn new(app_data: Option<PathBuf>) -> Self {
        Self {
            dir: std::env::var_os("NOUS_MAP_SCAN")
                .and(app_data)
                .map(|d| d.join(DIR)),
            floors: BTreeMap::new(),
            floor: None,
        }
    }

    pub fn on_command(&mut self, command: &GameCommand) {
        if self.dir.is_none() {
            return;
        }
        let changed = match command.command_id {
            cmd::EnterSceneByServerScNotify => command
                .parse_proto::<proto::EnterSceneByServerScNotify::EnterSceneByServerScNotify>()
                .ok()
                .and_then(|r| r.scene.into_option())
                .map(|s| self.scene(&s)),
            cmd::GetCurSceneInfoScRsp => command
                .parse_proto::<proto::GetCurSceneInfoScRsp::GetCurSceneInfoScRsp>()
                .ok()
                .and_then(|r| r.scene.into_option())
                .map(|s| self.scene(&s)),
            cmd::SceneGroupRefreshScNotify => command
                .parse_proto::<proto::SceneGroupRefreshScNotify::SceneGroupRefreshScNotify>()
                .ok()
                .map(|r| self.refresh(&r)),
            cmd::GetSceneMapInfoScRsp => command
                .parse_proto::<proto::GetSceneMapInfoScRsp::GetSceneMapInfoScRsp>()
                .ok()
                .map(|r| self.map(&r)),
            _ => None,
        };
        for floor in changed.unwrap_or_default() {
            self.save(floor);
        }
    }

    fn scene(&mut self, s: &proto::SceneInfo::SceneInfo) -> Vec<u32> {
        self.floor = Some(s.floor_id);
        let scan = self.floor_scan(s.floor_id);
        scan.plane = s.plane_id;
        for e in s
            .entity_list
            .iter()
            .chain(s.entity_group_list.iter().flat_map(|g| &g.entity_list))
        {
            add(scan, e);
        }
        vec![s.floor_id]
    }

    fn refresh(
        &mut self,
        r: &proto::SceneGroupRefreshScNotify::SceneGroupRefreshScNotify,
    ) -> Vec<u32> {
        use proto::SceneEntityRefreshInfo::scene_entity_refresh_info::Refresh_type;
        let Some(floor) = Some(r.floor_id).filter(|f| *f != 0).or(self.floor) else {
            return vec![];
        };
        let scan = self.floor_scan(floor);
        for e in r.group_refresh_list.iter().flat_map(|g| &g.refresh_entity) {
            if let Some(Refresh_type::AddEntity(e)) = &e.refresh_type {
                add(scan, e);
            }
        }
        vec![floor]
    }

    fn map(&mut self, r: &proto::GetSceneMapInfoScRsp::GetSceneMapInfoScRsp) -> Vec<u32> {
        r.scene_map_info
            .iter()
            .map(|i| {
                let scan = self.floor_scan(i.floor_id);
                scan.map = Some(MapReply {
                    chests: i
                        .chest_list
                        .iter()
                        .map(|c| [c.chest_type.value() as u32, c.exist_num, c.opened_num])
                        .collect(),
                    opened: i.lighten_section_list.clone(),
                    opened_ids: i.opened_chest_id_list.clone(),
                    props: i
                        .maze_prop_list
                        .iter()
                        .map(|p| (format!("{}:{}", p.group_id, p.config_id), p.state))
                        .collect(),
                    teleports: i.unlocked_teleport_list.clone(),
                    groups: i.maze_group_list.iter().map(|g| g.group_id).collect(),
                    saved: i.floor_saved_data.clone().into_iter().collect(),
                });
                i.floor_id
            })
            .collect()
    }

    /// The floor's scan, read from disk on first use so visits add up.
    fn floor_scan(&mut self, floor: u32) -> &mut FloorScan {
        let path = self.path(floor);
        self.floors.entry(floor).or_insert_with(|| {
            path.and_then(|p| fs::read(p).ok())
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or(FloorScan {
                    floor,
                    ..FloorScan::default()
                })
        })
    }

    fn path(&self, floor: u32) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(format!("{floor}.json")))
    }

    fn save(&self, floor: u32) {
        let (Some(path), Some(scan)) = (self.path(floor), self.floors.get(&floor)) else {
            return;
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).ok();
        }
        if let Ok(text) = serde_json::to_string_pretty(scan) {
            fs::write(path, text).ok();
        }
    }
}

fn add(scan: &mut FloorScan, e: &proto::SceneEntityInfo::SceneEntityInfo) {
    use proto::SceneEntityInfo::scene_entity_info::Entity as WireEntity;
    let (kind, id, event, state) = match &e.entity {
        Some(WireEntity::Prop(p)) => ("prop", p.prop_id, None, Some(p.prop_state)),
        Some(WireEntity::Npc(n)) => ("npc", n.npc_id, None, None),
        Some(WireEntity::NpcMonster(m)) => ("monster", m.monster_id, Some(m.event_id), None),
        _ => return,
    };
    let pos = e.motion.as_ref().and_then(|m| m.pos_index.as_ref());
    scan.objects.insert(
        format!("{}:{}", e.group_id, e.inst_id),
        Object {
            kind: kind.into(),
            id,
            event,
            state,
            x: pos.map_or(0, |p| p.x),
            y: pos.map_or(0, |p| p.y),
            z: pos.map_or(0, |p| p.z),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::Message;

    #[test]
    fn scene_objects_and_map_add_up_per_floor() {
        let dir = std::env::temp_dir().join(format!("nous-mapscan-test-{}", std::process::id()));
        let mut scan = MapScan {
            dir: Some(dir.clone()),
            floors: BTreeMap::new(),
            floor: None,
        };
        let mut prop = proto::SceneEntityInfo::SceneEntityInfo::new();
        prop.group_id = 7;
        prop.inst_id = 3;
        let mut info = proto::ScenePropInfo::ScenePropInfo::new();
        info.prop_id = 60001;
        info.prop_state = 12;
        prop.entity = Some(proto::SceneEntityInfo::scene_entity_info::Entity::Prop(
            info,
        ));
        let mut s = proto::SceneInfo::SceneInfo::new();
        s.floor_id = 20101001;
        s.plane_id = 20101;
        s.entity_list.push(prop);
        let mut rsp = proto::GetCurSceneInfoScRsp::GetCurSceneInfoScRsp::new();
        rsp.scene = Some(s).into();
        scan.on_command(&GameCommand {
            command_id: cmd::GetCurSceneInfoScRsp,
            header_len: 0,
            data_len: 0,
            proto_header: vec![],
            proto_data: rsp.write_to_bytes().unwrap(),
        });

        let saved: FloorScan =
            serde_json::from_slice(&fs::read(dir.join("20101001.json")).unwrap()).unwrap();
        assert_eq!(saved.plane, 20101);
        assert_eq!(saved.objects["7:3"].id, 60001);
        assert_eq!(saved.objects["7:3"].state, Some(12));

        // a new session reads the file back and adds to it
        let mut again = MapScan {
            dir: Some(dir.clone()),
            floors: BTreeMap::new(),
            floor: None,
        };
        again.floor_scan(20101001).plane = 1;
        assert_eq!(again.floors[&20101001].objects.len(), 1);
        fs::remove_dir_all(dir).ok();
    }
}
