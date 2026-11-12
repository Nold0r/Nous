use nous_core::game::ProfilePatch;
use nous_core::inventory::{Bag, Relic};
use nous_core::{Account, AccountStore, Client, Command, FileStore};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Default)]
struct MemoryStore {
    accounts: Arc<Mutex<BTreeMap<u32, Account>>>,
    fail: Arc<AtomicBool>,
}
impl AccountStore for MemoryStore {
    fn load_current(&self) -> Result<Account, String> {
        Ok(Account::empty())
    }
    fn load_or_new(&self, uid: u32) -> Result<Account, String> {
        Ok(self
            .accounts
            .lock()
            .unwrap()
            .get(&uid)
            .cloned()
            .unwrap_or_else(|| Account::new(uid)))
    }
    fn save(&self, account: &Account) -> Result<(), String> {
        if self.fail.load(Ordering::SeqCst) {
            return Err("test store unavailable".into());
        }
        if let Some(uid) = account.uid {
            self.accounts.lock().unwrap().insert(uid, account.clone());
        }
        Ok(())
    }
}
fn login(uid: u32) -> Command {
    Command {
        login: Some(uid),
        ..Default::default()
    }
}
fn bag() -> Command {
    Command {
        bag: Some(Bag {
            relics: vec![Relic {
                uid: 10,
                tid: 61011,
                ..Default::default()
            }],
            ..Default::default()
        }),
        ..Default::default()
    }
}
#[test]
fn snapshot_and_multiple_subscribers_observe_the_same_revision() {
    let client = Client::new(MemoryStore::default()).unwrap();
    let a = client.subscribe();
    let b = client.subscribe();
    assert_eq!(client.read().revision, 0);
    let revision = client.handle(&login(42)).unwrap();
    for name in ["account-changed", "profile-changed", "inventory-changed"] {
        let event = a.recv().unwrap();
        let copy = b.recv().unwrap();
        assert_eq!(event.name, name);
        assert_eq!(copy.payload, event.payload);
        assert_eq!(event.payload["revision"], revision);
    }
    let snapshot = client.read();
    assert_eq!(snapshot.revision, revision);
    assert_eq!(snapshot.data.uid, Some(42));
    let revision = client.handle(&bag()).unwrap();
    let event = a.recv().unwrap();
    assert_eq!(
        event.payload,
        serde_json::json!({"revision": revision, "data": {"kind": "reset"}})
    );
    assert_eq!(client.read().data.inventory.relics[&10].tid, 61011);
    assert_eq!(
        client.handle(&Command::default()).unwrap(),
        revision,
        "unknown commands do not advance state"
    );
}
#[test]
fn account_switches_and_shutdown_preserve_data_with_replaceable_storage() {
    let store = MemoryStore::default();
    let client = Client::new(store.clone()).unwrap();
    client.handle(&login(1)).unwrap();
    client.handle(&bag()).unwrap();
    client.handle(&login(2)).unwrap();
    assert!(client.read().data.inventory.relics.is_empty());
    assert!(
        store.accounts.lock().unwrap()[&1]
            .inventory
            .relics
            .contains_key(&10)
    );
    client.handle(&login(1)).unwrap();
    assert!(client.read().data.inventory.relics.contains_key(&10));
    let copy = client.clone();
    drop(client);
    copy.handle(&Command {
        profile: Some(ProfilePatch {
            nickname: Some("Тест".into()),
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();
    drop(copy);
    assert_eq!(
        store.accounts.lock().unwrap()[&1]
            .profile
            .nickname
            .as_deref(),
        Some("Тест")
    );
}
#[test]
fn failed_save_is_reported_and_retried_before_switching_accounts() {
    let store = MemoryStore::default();
    let client = Client::new(store.clone()).unwrap();
    let events = client.subscribe();
    client.handle(&login(1)).unwrap();
    client.handle(&bag()).unwrap();
    store.fail.store(true, Ordering::SeqCst);
    assert!(client.flush().unwrap_err().contains("unavailable"));
    let before = client.read().revision;
    assert!(client.handle(&login(2)).is_err());
    assert_eq!(client.read().data.uid, Some(1));
    assert_eq!(client.read().revision, before);
    assert!(events.try_iter().any(|e| e.name == "sdk-error"));
    store.fail.store(false, Ordering::SeqCst);
    client.handle(&login(2)).unwrap();
    assert!(
        store.accounts.lock().unwrap()[&1]
            .inventory
            .relics
            .contains_key(&10)
    );
}
#[test]
fn failed_load_does_not_destroy_the_current_account() {
    struct BadLoad;
    impl AccountStore for BadLoad {
        fn load_current(&self) -> Result<Account, String> {
            Ok(Account::new(1))
        }
        fn load_or_new(&self, _: u32) -> Result<Account, String> {
            Err("cannot read".into())
        }
        fn save(&self, _: &Account) -> Result<(), String> {
            Ok(())
        }
    }
    let client = Client::new(BadLoad).unwrap();
    assert!(client.handle(&login(2)).is_err());
    assert_eq!(client.read().data.uid, Some(1));
}
#[test]
fn filesystem_account_can_be_loaded_after_sdk_shutdown() {
    let dir = std::env::temp_dir().join(format!("nous-sdk-api-{}", std::process::id()));
    {
        let client = Client::new(FileStore::new(Some(dir.clone()))).unwrap();
        client.handle(&login(314)).unwrap();
        client.handle(&bag()).unwrap();
    }
    let fresh = Client::new(FileStore::new(Some(dir.clone()))).unwrap();
    assert_eq!(fresh.read().data.uid, Some(314));
    assert_eq!(fresh.read().data.format, 2);
    assert!(fresh.read().data.inventory.relics.contains_key(&10));
    drop(fresh);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn reads_and_packet_handling_continue_while_a_snapshot_is_being_saved() {
    use std::sync::mpsc;
    struct SlowStore {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }
    impl AccountStore for SlowStore {
        fn load_current(&self) -> Result<Account, String> {
            Ok(Account::new(1))
        }
        fn load_or_new(&self, uid: u32) -> Result<Account, String> {
            Ok(Account::new(uid))
        }
        fn save(&self, _: &Account) -> Result<(), String> {
            self.entered.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
            Ok(())
        }
    }
    let (entered, waiting) = mpsc::channel();
    let (release, released) = mpsc::channel();
    let client = Client::new(SlowStore {
        entered,
        release: Mutex::new(released),
    })
    .unwrap();
    client.handle(&bag()).unwrap();
    let saving = client.clone();
    let worker = std::thread::spawn(move || saving.flush().unwrap());
    waiting.recv().unwrap();
    assert!(client.read().data.inventory.relics.contains_key(&10));
    let revision = client
        .handle(&Command {
            profile: Some(ProfilePatch {
                level: Some(70),
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
    release.send(()).unwrap();
    worker.join().unwrap();
    // A change made during the write must still be dirty and saved next time.
    let saving = client.clone();
    let worker = std::thread::spawn(move || saving.flush().unwrap());
    waiting.recv().unwrap();
    release.send(()).unwrap();
    worker.join().unwrap();
    assert_eq!(client.read().revision, revision);
}

#[test]
fn a_multi_domain_command_is_one_revision_with_the_existing_event_envelopes() {
    use nous_core::world::{Entity, WorldCommand};
    let client = Client::new(MemoryStore::default()).unwrap();
    let events = client.subscribe();
    let mut command = bag();
    command.login = Some(77);
    command.world = Some(WorldCommand::Scene {
        plane: 1,
        floor: 2,
        entry: 3,
        leader: 4,
        entities: vec![Entity {
            id: 4,
            group: 0,
            config: 0,
            prop_state: None,
            position: None,
        }],
    });
    command.battle = Some(nous_core::battles::Battle {
        stage_id: 5,
        ..Default::default()
    });
    command.banners = Some(vec![nous_core::gacha::Banner {
        gacha_id: 6,
        ..Default::default()
    }]);
    command.snapshot = Some((
        "GetDailyActiveInfoScRsp".into(),
        nous_core::snapshots::Snapshot {
            at: 10,
            data: serde_json::json!({ "score": 20 }),
        },
    ));
    let revision = client.handle(&command).unwrap();
    assert_eq!(revision, 1);
    let snapshot = client.read();
    assert_eq!(snapshot.data.world.floor_id, Some(2));
    assert_eq!(snapshot.data.battles.list[0].stage_id, 5);
    assert_eq!(snapshot.data.gacha.banners[0].gacha_id, 6);
    assert_eq!(
        snapshot.data.snapshots["GetDailyActiveInfoScRsp"].data["score"],
        20
    );
    let emitted: Vec<_> = events.try_iter().collect();
    assert_eq!(
        emitted.iter().map(|e| e.name).collect::<Vec<_>>(),
        [
            "account-changed",
            "profile-changed",
            "inventory-changed",
            "world-changed",
            "battle-finished",
            "gacha-changed",
            "snapshot-changed"
        ]
    );
    assert!(
        emitted
            .iter()
            .all(|e| e.payload["revision"] == snapshot.revision)
    );
    assert_eq!(emitted[0].payload["data"], 77);
    assert_eq!(emitted[5].payload["data"], serde_json::Value::Null);
    assert_eq!(emitted[6].payload["data"], "GetDailyActiveInfoScRsp");
}

#[test]
fn relic_notes_are_saved_with_the_account_and_published() {
    let store = MemoryStore::default();
    let client = Client::new(store.clone()).unwrap();
    assert!(client.set_relic_note(10, "на Ахерон").is_err(), "no account yet");
    client.handle(&login(7)).unwrap();
    let events = client.subscribe();
    client.set_relic_note(10, "  на Ахерон ").unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event.name, "notes-changed");
    assert_eq!(event.payload["data"]["relics"]["10"]["text"], "на Ахерон");
    client.set_relic_note(10, "на Ахерон").unwrap();
    assert!(events.try_recv().is_err(), "same text publishes nothing");
    client.flush().unwrap();
    assert_eq!(store.accounts.lock().unwrap()[&7].notes.relics[&10].text, "на Ахерон");
    client.set_relic_note(10, "").unwrap();
    assert!(client.read().data.notes.relics.is_empty());
    assert!(events.try_recv().is_ok(), "deleting publishes too");
    client.set_character_note(1308, "E2").unwrap();
    assert_eq!(events.try_recv().unwrap().payload["data"]["characters"]["1308"]["text"], "E2");
}

#[test]
fn first_import_is_not_new_later_drops_are() {
    use nous_core::inventory::InventorySync;
    let client = Client::new(MemoryStore::default()).unwrap();
    client.handle(&login(7)).unwrap();
    client.handle(&bag()).unwrap();
    let first = client.read().data.inventory;
    let imported = first.imported_at.expect("the first list marks what was already owned");
    assert_eq!(first.relics[&10].first_seen, imported);

    // a drop while playing reaches the UI right away and is newer than the import
    let events = client.subscribe();
    client
        .handle(&Command {
            inventory_sync: Some(InventorySync {
                relics: vec![Relic { uid: 11, tid: 61012, ..Default::default() }],
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event.name, "inventory-changed");
    assert_eq!(event.payload["data"]["relics"][0]["uid"], 11);
    client.handle(&bag()).unwrap();
    assert_eq!(client.read().data.inventory.imported_at, Some(imported), "later logins don't move it");
}

fn prop(id: u32, state: u32) -> nous_core::world::Entity {
    nous_core::world::Entity { id, group: 5, config: id, prop_state: Some(state), position: None }
}
fn world(command: nous_core::world::WorldCommand) -> Command {
    Command { world: Some(command), ..Default::default() }
}

#[test]
fn world_changed_carries_only_the_floors_that_changed() {
    use nous_core::world::{FloorMap, WorldCommand};
    let client = Client::new(MemoryStore::default()).unwrap();
    client.handle(&login(7)).unwrap();
    client
        .handle(&world(WorldCommand::Map(vec![FloorMap {
            floor: 20,
            chests: [1, 2].into(),
            props: vec![(5, 9, 8)],
        }])))
        .unwrap();
    let events = client.subscribe();
    let scene = WorldCommand::Scene { plane: 1, floor: 10, entry: 3, leader: 4, entities: vec![prop(1, 12)] };
    let revision = client.handle(&world(scene)).unwrap();
    let event = events.try_recv().unwrap();
    assert_eq!(event.name, "world-changed");
    assert_eq!(event.payload["revision"], revision);
    let data = &event.payload["data"];
    assert_eq!(data["floorId"], 10);
    assert_eq!(data["planeId"], 1);
    assert_eq!(
        data["floors"],
        serde_json::json!({ "10": { "openedChests": [], "openedAt": {}, "props": { "5:1": 12 } } }),
        "floor 20 did not change and is not sent again"
    );

    // the game repeats what Nous already knows: no event, no new revision
    let same = WorldCommand::Refresh { floor: 10, entities: vec![prop(1, 12)] };
    assert_eq!(client.handle(&world(same)).unwrap(), revision);
    assert!(events.try_recv().is_err());
}

#[test]
fn the_place_where_the_player_stopped_is_delivered() {
    use nous_core::world::{Position, WorldCommand};
    let at = |x| Position { x, y: 0, z: 0, rot_y: 0, layer: Some(0) };
    let client = Client::new(MemoryStore::default()).unwrap();
    client.handle(&login(7)).unwrap();
    let events = client.subscribe();
    let first = client.handle(&world(WorldCommand::Move(vec![(4, at(1))]))).unwrap();
    client.handle(&world(WorldCommand::Move(vec![(4, at(2))]))).unwrap();

    let event = events.recv_timeout(std::time::Duration::from_secs(2)).unwrap();
    assert_eq!((event.name, &event.payload["data"]["x"]), ("player-moved", &serde_json::json!(1)));
    let late = events
        .recv_timeout(std::time::Duration::from_secs(2))
        .expect("the last position arrives without another move");
    assert_eq!(late.name, "player-moved");
    assert_eq!(late.payload["data"]["x"], 2);
    assert!(late.payload["revision"].as_u64().unwrap() > first);
    assert_eq!(client.read().revision, late.payload["revision"].as_u64().unwrap());
}
