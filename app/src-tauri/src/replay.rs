//! Data tests on real logins, without the game: replays the login recordings
//! (recorder.rs) through the inventory code and checks the result.
//!
//!   npm run test:data            (from app/)
//!
//! Recordings come from `%APPDATA%\com.nous.companion\recordings`, or the
//! folder in NOUS_RECORDINGS (hsr-sniff dumps work too). With none, the test
//! passes and says so: CI has no account data.

use std::collections::BTreeMap;
use std::path::PathBuf;

use base64::Engine;
use protobuf::Message;
use reliquary::network::command::proto;

use crate::inventory::{Inventory, SnapshotReport};

struct Session {
    file: String,
    world: crate::world::World,
    snapshots: crate::snapshots::Snapshots,
    gacha: crate::gacha::Gacha,
    battles: crate::battles::Battles,
    uid: u32,
    inventory: Inventory,
    had_bag: bool,
    had_avatars: bool,
}

fn recordings_dir() -> Option<PathBuf> {
    std::env::var_os("NOUS_RECORDINGS")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("com.nous.companion").join(crate::recorder::DIR)))
}

fn replay_file(path: &std::path::Path) -> Session {
    let mut s = Session {
        file: path.file_name().unwrap().to_string_lossy().into_owned(),
        world: Default::default(),
        snapshots: Default::default(),
        gacha: Default::default(),
        battles: Default::default(),
        uid: 0,
        inventory: Inventory::default(),
        had_bag: false,
        had_avatars: false,
    };
    let text = std::fs::read_to_string(path).unwrap();
    for (n, line) in text.lines().enumerate() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let (Some(name), Some(b64)) = (v["name"].as_str(), v["b64"].as_str()) else { continue };
        let data = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let now = n as i64;
        if let Some(id) = v["id"].as_u64() {
            let command = reliquary::network::command::GameCommand {
                command_id: id as u16,
                header_len: 0,
                data_len: data.len() as u32,
                proto_header: vec![],
                proto_data: data.clone(),
            };
            s.world.handle(&command);
            s.battles.handle(&command, now);
            s.gacha.handle(&command, now);
            crate::snapshots::handle(&mut s.snapshots, &command, now);
        }
        match name {
            "PlayerGetTokenScRsp" => {
                s.uid = proto::PlayerGetTokenScRsp::PlayerGetTokenScRsp::parse_from_bytes(&data).unwrap().uid;
            }
            "GetBagScRsp" => {
                let bag = proto::GetBagScRsp::GetBagScRsp::parse_from_bytes(&data).expect("GetBagScRsp parses");
                s.inventory.apply_bag(&bag, now);
                s.had_bag = true;
            }
            "GetAvatarDataScRsp" => {
                let data = proto::GetAvatarDataScRsp::GetAvatarDataScRsp::parse_from_bytes(&data).expect("GetAvatarDataScRsp parses");
                s.inventory.apply_avatars(&data);
                s.had_avatars = true;
            }
            "PlayerSyncScNotify" => {
                let sync = proto::PlayerSyncScNotify::PlayerSyncScNotify::parse_from_bytes(&data).expect("PlayerSyncScNotify parses");
                s.inventory.apply_sync(&sync, now);
            }
            _ => {}
        }
    }
    s
}

/// Sanity of one login: lists present and consistent with each other.
fn check_session(s: &Session) -> Vec<String> {
    let inv = &s.inventory;
    let mut problems = Vec::new();
    let mut fail = |cond: bool, msg: String| {
        if cond {
            problems.push(msg)
        }
    };

    fail(s.uid == 0, "no PlayerGetTokenScRsp".into());
    fail(!s.had_bag, "no GetBagScRsp (login not fully recorded?)".into());
    fail(!s.had_avatars, "no GetAvatarDataScRsp".into());
    fail(s.had_bag && inv.relics.is_empty(), "bag has no relics".into());
    fail(s.had_avatars && inv.avatar_paths.is_empty(), "no characters".into());

    for r in inv.relics.values() {
        fail(r.tid == 0 || r.main_affix_id == 0, format!("relic {}: empty tid/main stat", r.uid));
        fail(r.level > 15, format!("relic {}: level {}", r.uid, r.level));
        fail(r.sub_affixes.len() > 4, format!("relic {}: {} substats", r.uid, r.sub_affixes.len()));
        fail(r.sub_affixes.iter().any(|a| a.cnt == 0), format!("relic {}: substat with 0 rolls", r.uid));
    }
    // gear worn by a character must exist, and point back at that character
    for path in inv.avatar_paths.values() {
        for (slot, uid) in &path.relics {
            match inv.relics.get(uid) {
                None => fail(true, format!("character {} slot {slot}: relic {uid} not in bag", path.avatar_id)),
                Some(r) => fail(
                    r.equipped_by != Some(path.avatar_id),
                    format!("relic {uid} worn by {} but says {:?}", path.avatar_id, r.equipped_by),
                ),
            }
        }
        if let Some(lc) = path.light_cone {
            fail(!inv.light_cones.contains_key(&lc), format!("character {}: light cone {lc} not in bag", path.avatar_id));
        }
    }
    problems
}

/// Compares relic lists of two logins of the same account: a relic id must
/// never start meaning another relic.
fn compare(prev: &Session, next: &Session) -> SnapshotReport {
    let mut inv = prev.inventory.clone();
    let mut bag = proto::GetBagScRsp::GetBagScRsp::new();
    bag.relic_list = next
        .inventory
        .relics
        .values()
        .map(|r| {
            let mut p = proto::Relic::Relic::new();
            p.unique_id = r.uid;
            p.tid = r.tid;
            p.main_affix_id = r.main_affix_id;
            p.level = r.level;
            p.exp = r.exp;
            p.equip_avatar_id = r.equipped_by.unwrap_or_default();
            p.is_protected = r.locked;
            p.is_discarded = r.discarded;
            p.sub_affix_list = to_proto(&r.sub_affixes);
            p.reforge_sub_affix_list = to_proto(&r.reforge_sub_affixes);
            p.preview_sub_affix_list = to_proto(&r.preview_sub_affixes);
            p
        })
        .collect();
    inv.apply_bag(&bag, i64::MAX)
}

fn to_proto(affixes: &[crate::inventory::Affix]) -> Vec<proto::RelicAffix::RelicAffix> {
    affixes
        .iter()
        .map(|a| {
            let mut x = proto::RelicAffix::RelicAffix::new();
            x.affix_id = a.affix_id;
            x.cnt = a.cnt;
            x.step = a.step;
            x
        })
        .collect()
}

#[test]
fn replay_login_recordings() {
    let Some(dir) = recordings_dir().filter(|d| d.is_dir()) else {
        println!("no login recordings: log in to the game once with Nous running");
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    files.sort();
    if files.is_empty() {
        println!("no login recordings in {}", dir.display());
        return;
    }

    let mut failures = Vec::new();
    let mut last_by_uid: BTreeMap<u32, Session> = BTreeMap::new();
    for path in &files {
        let s = replay_file(path);
        let inv = &s.inventory;
        println!(
            "{}: uid {}, {} relics, {} light cones, {} materials, {} characters",
            s.file,
            s.uid,
            inv.relics.len(),
            inv.light_cones.len(),
            inv.materials.len(),
            inv.avatar_paths.len()
        );
        println!(
            "  world: floor {:?}, player {}, {} floors with props; banners {}; battles {}; snapshots {:?}",
            s.world.floor_id,
            s.world.player.is_some(),
            s.world.props.len(),
            s.gacha.banners.len(),
            s.battles.list.len(),
            s.snapshots.keys().collect::<Vec<_>>()
        );
        for p in check_session(&s) {
            failures.push(format!("{}: {p}", s.file));
        }
        if let Some(prev) = last_by_uid.get(&s.uid).filter(|p| p.had_bag && s.had_bag) {
            let report = compare(prev, &s);
            println!("  vs {}: {report:?}", prev.file);
            if report.reassigned > 0 {
                failures.push(format!("{}: {} relic ids now point at other relics (vs {})", s.file, report.reassigned, prev.file));
            }
        }
        last_by_uid.insert(s.uid, s);
    }
    assert!(failures.is_empty(), "data problems:\n{}", failures.join("\n"));
}
