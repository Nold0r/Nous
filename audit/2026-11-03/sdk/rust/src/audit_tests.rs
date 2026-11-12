//! Safety properties checked through the original SDK path dependencies.
//! FAIL means the property is violated in the audited application.
use nous_core::{AccountStore, FileStore as Store};
use std::collections::BTreeMap;
use std::fs;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
struct GameState(nous_core::GameState);
impl Deref for GameState {
    type Target = nous_core::GameState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for GameState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl GameState {
    fn new(s: Store) -> Self {
        Self(nous_core::GameState::new(s).unwrap())
    }
    fn handle(&mut self, c: &GameCommand) {
        self.0.handle(&nous_hsr::decode(c)).unwrap();
    }
    fn flush(&mut self) {
        self.0.flush().unwrap();
    }
}

use reliquary::network::{
    GameSniffer,
    command::{GameCommand, command_id as cmd, proto},
};

fn store(name: &str) -> (Store, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../results/rust-fixtures")
        .join(format!("{}-{name}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    (Store::new(Some(dir.clone())), dir)
}

fn command<M: protobuf::Message>(id: u16, msg: &M) -> GameCommand {
    let data = msg.write_to_bytes().unwrap();
    GameCommand {
        command_id: id,
        header_len: 0,
        data_len: data.len() as u32,
        proto_header: vec![],
        proto_data: data,
    }
}
fn token(uid: u32) -> GameCommand {
    let mut p = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
    p.uid = uid;
    command(cmd::PlayerGetTokenScRsp, &p)
}
fn bag() -> GameCommand {
    let mut p = proto::GetBagScRsp::GetBagScRsp::new();
    let mut r = proto::Relic::Relic::new();
    r.unique_id = 7;
    r.tid = 61011;
    r.main_affix_id = 1;
    p.relic_list.push(r);
    command(cmd::GetBagScRsp, &p)
}

#[test]
fn failed_bag_response_must_not_erase_inventory() {
    let (s, _) = store("failed-bag");
    let mut g = GameState::new(s);
    g.handle(&token(111));
    g.handle(&bag());
    let mut failed = proto::GetBagScRsp::GetBagScRsp::new();
    failed.retcode = 1;
    g.handle(&command(cmd::GetBagScRsp, &failed));
    assert_eq!(
        g.account.inventory.relics.len(),
        1,
        "failed response erased a valid bag"
    );
}

#[test]
fn failed_token_response_must_not_switch_to_uid_zero() {
    let (s, _) = store("failed-token");
    let mut g = GameState::new(s);
    g.handle(&token(111));
    let mut failed = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::new();
    failed.retcode = 1;
    g.handle(&command(cmd::PlayerGetTokenScRsp, &failed));
    assert_eq!(
        g.account.uid,
        Some(111),
        "rejected authentication replaced the account with UID 0"
    );
}

#[test]
fn packets_before_new_token_must_not_mutate_cached_account() {
    let (s, _) = store("before-token");
    let mut g = GameState::new(s.clone());
    g.handle(&token(111));
    g.flush();
    let mut restarted = GameState::new(s);
    restarted.handle(&bag()); // could be another conversation/account, UID not established
    assert!(
        restarted.account.inventory.relics.is_empty(),
        "unattributed packets mutated the last account"
    );
}

#[test]
fn account_file_uid_must_match_requested_uid() {
    let (s, dir) = store("uid-mismatch");
    let a = s.load_or_new(222).unwrap();
    s.save(&a).unwrap();
    fs::copy(dir.join("accounts/222.json"), dir.join("accounts/111.json")).unwrap();
    assert_eq!(
        s.load_or_new(111).unwrap().uid,
        Some(111),
        "filename UID and embedded UID are not checked"
    );
}

#[test]
fn nonfinite_battle_stat_must_not_make_entire_account_unreadable() {
    let (s, _) = store("nan-stat");
    let mut g = GameState::new(s.clone());
    g.handle(&token(111));
    g.handle(&bag());
    let mut stt = proto::BattleStatistics::BattleStatistics::new();
    stt.cost_time = f64::NAN;
    let mut req = proto::PVEBattleResultCsReq::PVEBattleResultCsReq::new();
    req.stt = Some(stt).into();
    g.handle(&command(cmd::PVEBattleResultCsReq, &req));
    g.flush();
    assert_eq!(
        s.load_or_new(111).unwrap().inventory.relics.len(),
        1,
        "NaN serialized to null and made all account data unreadable"
    );
}

#[test]
fn last_unrelated_cache_url_must_not_hide_valid_history_url() {
    let (s, dir) = store("cache-url");
    drop(s);
    let cache = dir.join("StarRail_Data/webCaches/2.54.0.0/Cache/Cache_Data");
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join("data_2"), b"https://public-operation-hkrpg-sg.hoyoverse.com/common/hkrpg_gacha_record/api/getGachaLog?authkey=TEST\0https://public-operation-hkrpg-sg.hoyoverse.com/unrelated\0").unwrap();
    assert!(
        nous_hsr::gacha::find_log_url(&dir.join("StarRail.exe")).is_ok(),
        "rfind selects a non-history URL and never checks older matches"
    );
}

#[test]
fn import_must_reconcile_live_pulls_already_in_history() {
    let mut g = nous_core::gacha::Gacha::default();
    g.live.push(nous_core::gacha::LivePull {
        gacha_id: 1,
        item_id: 1310,
        is_new: true,
        at: 100,
    });
    g.merge(
        vec![nous_core::gacha::Pull {
            id: "123".into(),
            gacha_id: 1,
            item_id: 1310,
            time: "2026-11-03 07:00:00".into(),
            ..Default::default()
        }],
        200,
    );
    assert!(
        g.live.is_empty(),
        "live pull still counted separately after official history import"
    );
}

// Build ordinary Ethernet / IPv4 / UDP frames without sending them anywhere.
fn udp(payload: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 42];
    b[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
    b[14] = 0x45;
    b[16..18].copy_from_slice(&((28 + payload.len()) as u16).to_be_bytes());
    b[22] = 64;
    b[23] = 17;
    b[26..30].copy_from_slice(&[127, 0, 0, 1]);
    b[30..34].copy_from_slice(&[127, 0, 0, 1]);
    b[34..36].copy_from_slice(&23301u16.to_be_bytes());
    b[36..38].copy_from_slice(&50000u16.to_be_bytes());
    b[38..40].copy_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
    b.extend_from_slice(payload);
    b
}
fn kcp(payload: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; 28];
    b[0..4].copy_from_slice(&1u32.to_le_bytes());
    b[8] = 81;
    b[10..12].copy_from_slice(&1024u16.to_le_bytes());
    b[24..28].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    b.extend_from_slice(payload);
    b
}

#[test]
fn short_udp_handshake_must_return_error_instead_of_panicking() {
    for len in 4..8 {
        let result =
            std::panic::catch_unwind(|| GameSniffer::new().receive_packet(udp(&vec![0; len])));
        assert!(
            result.is_ok(),
            "a {len}-byte UDP payload panics and kills the capture task"
        );
    }
}

#[test]
fn short_kcp_command_must_return_error_instead_of_panicking() {
    let result = std::panic::catch_unwind(|| GameSniffer::new().receive_packet(udp(&kcp(&[0]))));
    assert!(
        result.is_ok(),
        "1-byte reassembled command panics in get_game_version"
    );
}

#[test]
fn malformed_token_protobuf_must_return_error_instead_of_panicking() {
    let mut raw = 0x9D74C714u32.to_be_bytes().to_vec();
    raw.extend_from_slice(&cmd::PlayerGetTokenScRsp.to_be_bytes());
    raw.extend_from_slice(&0u16.to_be_bytes());
    raw.extend_from_slice(&1u32.to_be_bytes());
    raw.push(0xff);
    raw.extend_from_slice(&[0; 4]);
    let keys = [(0u32, vec![0u8])].into_iter().collect();
    let result = std::panic::catch_unwind(|| {
        GameSniffer::new()
            .set_initial_keys(keys)
            .receive_packet(udp(&kcp(&raw)))
    });
    assert!(
        result.is_ok(),
        "token protobuf parse unwrap kills capture task"
    );
}

#[test]
fn normal_store_roundtrip_control() {
    let (s, _) = store("control");
    let mut g = GameState::new(s.clone());
    g.handle(&token(111));
    g.handle(&bag());
    g.flush();
    assert_eq!(s.load_or_new(111).unwrap().inventory.relics.len(), 1);
}

#[test]
fn normal_gacha_merge_control() {
    let mut g = nous_core::gacha::Gacha::default();
    assert_eq!(
        g.merge(
            vec![nous_core::gacha::Pull {
                id: "1".into(),
                ..Default::default()
            }],
            1
        ),
        1
    );
    assert_eq!(
        g.merge(
            vec![nous_core::gacha::Pull {
                id: "1".into(),
                ..Default::default()
            }],
            2
        ),
        0
    );
    assert_eq!(g.history.len(), 1);
    let _: BTreeMap<String, nous_core::gacha::Pull> = g.history;
}

fn entity(id: u32, x: i32) -> proto::SceneEntityInfo::SceneEntityInfo {
    let mut v = proto::Vector::Vector::new();
    v.x = x;
    let mut m = proto::MotionInfo::MotionInfo::new();
    m.pos_index = Some(v).into();
    let mut e = proto::SceneEntityInfo::SceneEntityInfo::new();
    e.entity_id = id;
    e.motion = Some(m).into();
    e
}
fn scene(world: &mut nous_core::world::World, s: proto::SceneInfo::SceneInfo) {
    let mut r = proto::GetCurSceneInfoScRsp::GetCurSceneInfoScRsp::new();
    r.scene = Some(s).into();
    world.handle(
        &nous_hsr::decode(&command(cmd::GetCurSceneInfoScRsp, &r))
            .world
            .unwrap(),
    );
}

#[test]
fn player_in_grouped_scene_entities_must_be_located() {
    let mut w = nous_core::world::World::default();
    let mut s = proto::SceneInfo::SceneInfo::new();
    s.leader_entity_id = 10;
    s.floor_id = 1;
    let mut group = proto::SceneEntityGroupInfo::SceneEntityGroupInfo::new();
    group.entity_list.push(entity(10, 123));
    s.entity_group_list.push(group);
    scene(&mut w, s);
    assert_eq!(
        w.player.map(|p| p.x),
        Some(123),
        "player lookup only checks flat entity_list"
    );
}

#[test]
fn changing_floor_must_not_reuse_previous_floor_layer() {
    let mut w = nous_core::world::World::default();
    w.floor_id = Some(1);
    w.player = Some(nous_core::world::Position {
        x: 0,
        y: 0,
        z: 0,
        rot_y: 0,
        layer: Some(9),
    });
    let mut s = proto::SceneInfo::SceneInfo::new();
    s.leader_entity_id = 10;
    s.floor_id = 2;
    s.entity_list.push(entity(10, 123));
    scene(&mut w, s);
    assert_eq!(
        w.player.unwrap().layer,
        None,
        "old floor's map layer leaked into a different floor"
    );
}

#[test]
fn movement_without_known_leader_must_not_move_player_to_other_entity() {
    let mut w = nous_core::world::World::default();
    let mut s = proto::SceneInfo::SceneInfo::new();
    s.leader_entity_id = 10;
    s.floor_id = 1;
    s.entity_list.push(entity(10, 123));
    scene(&mut w, s);
    let mut r = proto::SceneEntityMoveCsReq::SceneEntityMoveCsReq::new();
    let mut other = proto::EntityMotion::EntityMotion::new();
    other.entity_id = 99;
    other.motion = entity(99, 999).motion;
    r.entity_motion_list.push(other);
    w.handle(
        &nous_hsr::decode(&command(cmd::SceneEntityMoveCsReq, &r))
            .world
            .unwrap(),
    );
    assert_eq!(
        w.player.unwrap().x,
        123,
        "first mover fallback overwrote the known player's position"
    );
}

#[test]
fn failed_prop_interaction_must_not_change_saved_prop_state() {
    let mut w = nous_core::world::World::default();
    let mut s = proto::SceneInfo::SceneInfo::new();
    s.floor_id = 1;
    let mut e = entity(10, 123);
    e.group_id = 2;
    e.inst_id = 3;
    let mut prop = proto::ScenePropInfo::ScenePropInfo::new();
    prop.prop_state = 12;
    e.set_prop(prop);
    s.entity_list.push(e);
    scene(&mut w, s);
    let mut r = proto::InteractPropScRsp::InteractPropScRsp::new();
    r.retcode = 1;
    r.prop_entity_id = 10;
    w.handle(
        &nous_hsr::decode(&command(cmd::InteractPropScRsp, &r))
            .world
            .unwrap(),
    );
    assert_eq!(
        w.props[&1]["2:3"], 12,
        "failed interaction changed prop state to the default 0"
    );
}
