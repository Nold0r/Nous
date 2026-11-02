//! Data tests on real logins, without the game: replays the login recordings
//! (recorder.rs) through `GameState::handle` (the same path as the sniffer)
//! and checks the result, including that the account file reads back.
//!
//!   npm run test:data            (from app/)
//!
//! Recordings come from `%APPDATA%\com.nous.companion\recordings`, or the
//! folder in NOUS_RECORDINGS (hsr-sniff dumps work too). With none, the test
//! passes and says so: CI has no account data.

use std::collections::BTreeMap;
use std::path::PathBuf;

use base64::Engine;
use reliquary::network::command::proto;

use nous_core::inventory::{Inventory, SnapshotReport};

struct Session {
    file: String,
    world: nous_core::world::World,
    snapshots: nous_core::snapshots::Snapshots,
    gacha: nous_core::gacha::Gacha,
    battles: nous_core::battles::Battles,
    uid: u32,
    inventory: Inventory,
    had_bag: bool,
    had_avatars: bool,
}

fn recordings_dir() -> Option<PathBuf> {
    std::env::var_os("NOUS_RECORDINGS")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("APPDATA").map(|d| {
                PathBuf::from(d)
                    .join("com.nous.companion")
                    .join(crate::recorder::DIR)
            })
        })
}

/// Feeds a recording through the real path, `GameState::handle`, over a
/// throwaway store, then writes the account and reads it back.
fn replay_file(path: &std::path::Path) -> Session {
    let file = path.file_name().unwrap().to_string_lossy().into_owned();
    let dir = std::env::temp_dir().join(format!("nous-replay-{}-{file}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let store = nous_core::account::FileStore::new(Some(dir.clone()));
    let mut game = nous_core::game::GameState::new(store.clone()).unwrap();
    let (mut had_bag, mut had_avatars) = (false, false);

    let text = std::fs::read_to_string(path).unwrap();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let (Some(id), Some(b64)) = (v["id"].as_u64(), v["b64"].as_str()) else {
            continue;
        };
        let data = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        let command = reliquary::network::command::GameCommand {
            command_id: id as u16,
            header_len: 0,
            data_len: data.len() as u32,
            proto_header: vec![],
            proto_data: data,
        };
        match command.get_command_name() {
            Some("GetBagScRsp") => had_bag = true,
            Some("GetAvatarDataScRsp") => had_avatars = true,
            _ => {}
        }
        game.handle(&crate::decode(&command)).unwrap();
    }

    // what was recorded must survive a write and a fresh start
    game.flush().unwrap();
    let reloaded = nous_core::game::GameState::new(store).unwrap();
    assert_eq!(
        reloaded.account.inventory.relics.len(),
        game.account.inventory.relics.len(),
        "{file}: the account file reads back"
    );
    std::fs::remove_dir_all(&dir).ok();

    let a = game.account;
    Session {
        file,
        uid: a.uid.unwrap_or(0),
        inventory: a.inventory,
        world: a.world,
        snapshots: a.snapshots,
        gacha: a.gacha,
        battles: a.battles,
        had_bag,
        had_avatars,
    }
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
    fail(
        !s.had_bag,
        "no GetBagScRsp (login not fully recorded?)".into(),
    );
    fail(!s.had_avatars, "no GetAvatarDataScRsp".into());
    // A new account can legitimately have an empty bag. Gear references below
    // still detect missing items; presence of GetBagScRsp proves completeness.
    fail(
        s.had_avatars && inv.avatar_paths.is_empty(),
        "no characters".into(),
    );

    for r in inv.relics.values() {
        fail(
            r.tid == 0 || r.main_affix_id == 0,
            format!("relic {}: empty tid/main stat", r.uid),
        );
        fail(r.level > 15, format!("relic {}: level {}", r.uid, r.level));
        fail(
            r.sub_affixes.len() > 4,
            format!("relic {}: {} substats", r.uid, r.sub_affixes.len()),
        );
        fail(
            r.sub_affixes.iter().any(|a| a.cnt == 0),
            format!("relic {}: substat with 0 rolls", r.uid),
        );
    }
    // gear worn by a character must exist, and point back at that character
    for path in inv.avatar_paths.values() {
        for (slot, uid) in &path.relics {
            match inv.relics.get(uid) {
                None => fail(
                    true,
                    format!(
                        "character {} slot {slot}: relic {uid} not in bag",
                        path.avatar_id
                    ),
                ),
                Some(r) => fail(
                    r.equipped_by != Some(path.avatar_id),
                    format!(
                        "relic {uid} worn by {} but says {:?}",
                        path.avatar_id, r.equipped_by
                    ),
                ),
            }
        }
        if let Some(lc) = path.light_cone {
            fail(
                !inv.light_cones.contains_key(&lc),
                format!("character {}: light cone {lc} not in bag", path.avatar_id),
            );
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
    inv.apply_bag(&crate::inventory::bag(&bag), i64::MAX)
}

fn to_proto(affixes: &[nous_core::inventory::Affix]) -> Vec<proto::RelicAffix::RelicAffix> {
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
                failures.push(format!(
                    "{}: {} relic ids now point at other relics (vs {})",
                    s.file, report.reassigned, prev.file
                ));
            }
        }
        last_by_uid.insert(s.uid, s);
    }
    assert!(
        failures.is_empty(),
        "data problems:\n{}",
        failures.join("\n")
    );
}

#[test]
fn a_new_account_may_have_no_relics_but_must_have_a_complete_login() {
    let mut inventory = Inventory::default();
    inventory.avatar_paths.insert(
        8001,
        nous_core::inventory::AvatarPath {
            avatar_id: 8001,
            ..Default::default()
        },
    );
    let mut session = Session {
        file: "new-account-fixture".into(),
        uid: 1,
        inventory,
        world: Default::default(),
        snapshots: Default::default(),
        gacha: Default::default(),
        battles: Default::default(),
        had_bag: true,
        had_avatars: true,
    };
    assert!(
        check_session(&session).is_empty(),
        "an empty relic list is valid at the start of the game"
    );
    session.had_bag = false;
    assert!(
        check_session(&session)
            .iter()
            .any(|e| e.contains("no GetBagScRsp"))
    );
    session.had_bag = true;
    session
        .inventory
        .avatar_paths
        .get_mut(&8001)
        .unwrap()
        .relics
        .insert(1, 99);
    assert!(
        check_session(&session)
            .iter()
            .any(|e| e.contains("relic 99 not in bag")),
        "equipped items must still exist"
    );
}
