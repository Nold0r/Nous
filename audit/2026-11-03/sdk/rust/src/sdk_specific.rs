use nous_core::{Account, AccountStore, Client, Command, FileStore, GameState};
use serde_json::json;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn directory(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../results/rust-fixtures")
        .join(format!("{}-{name}", std::process::id()));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn login(uid: u32) -> Command {
    Command {
        login: Some(uid),
        ..Default::default()
    }
}

#[test]
fn failed_write_is_retried_control() {
    let root = directory("retry-control");
    let store = FileStore::new(Some(root.clone()));
    let mut g = GameState::new(store.clone()).unwrap();
    g.handle(&login(111)).unwrap();
    std::fs::write(root.join("accounts"), "not a directory").unwrap();
    assert!(g.flush().is_err());
    std::fs::remove_file(root.join("accounts")).unwrap();
    g.flush().unwrap();
    assert_eq!(store.load_current().unwrap().uid, Some(111));
}

#[test]
fn malformed_current_pointer_should_not_prevent_opening_the_application() {
    let root = directory("current-corrupt");
    std::fs::create_dir_all(root.join("accounts")).unwrap();
    std::fs::write(root.join("accounts/current"), "broken UID pointer").unwrap();
    let result = Client::new(FileStore::new(Some(root)));
    assert!(
        result.is_ok(),
        "a bad current pointer aborts SDK construction and Tauri setup"
    );
}

#[derive(Default)]
struct PartialLogStore {
    logs: Mutex<Vec<serde_json::Value>>,
    calls: AtomicUsize,
}
impl AccountStore for PartialLogStore {
    fn load_current(&self) -> Result<Account, String> {
        Ok(Account::new(111))
    }
    fn load_or_new(&self, uid: u32) -> Result<Account, String> {
        Ok(Account::new(uid))
    }
    fn save(&self, _: &Account) -> Result<(), String> {
        Ok(())
    }
    fn append_log(&self, _: u32, line: &serde_json::Value) -> Result<(), String> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 1 {
            return Err("simulated failure on second append".into());
        }
        self.logs.lock().unwrap().push(line.clone());
        Ok(())
    }
}
#[test]
fn retry_after_partial_log_write_must_not_duplicate_already_written_entries() {
    let store = Arc::new(PartialLogStore::default());
    let mut g = GameState::with_store(store.clone()).unwrap();
    for uid in [7, 8] {
        let mut sync = nous_core::inventory::InventorySync::default();
        sync.relics.push(nous_core::inventory::Relic {
            uid,
            tid: 61011,
            ..Default::default()
        });
        g.handle(&Command {
            inventory_sync: Some(sync),
            ..Default::default()
        })
        .unwrap();
    }
    assert!(g.flush().is_err());
    g.flush().unwrap();
    assert_eq!(
        store.logs.lock().unwrap().len(),
        2,
        "the first successful append was replayed after failure of the second"
    );
}

#[test]
fn no_change_result_from_veritas_must_not_mutate_state() {
    let mut e = nous_veritas::Engine::default();
    let before = serde_json::to_value(e.snapshot()).unwrap();
    assert!(!e.apply("OnDamage",json!({"attacker":{"uid":9,"team":"Enemy"},"damage":10,"overkill_damage":0,"type":"Normal"})).unwrap());
    assert_eq!(
        serde_json::to_value(e.snapshot()).unwrap(),
        before,
        "ignored enemy damage created a hidden active battle without a revision"
    );
}

#[test]
fn veritas_must_not_accept_negative_damage() {
    let mut e = nous_veritas::Engine::default();
    let r=e.apply("OnDamage",json!({"attacker":{"uid":1002,"team":"Player"},"damage":-100,"overkill_damage":0,"type":"Normal"}));
    assert!(
        r.is_err(),
        "negative network input becomes negative total damage"
    );
}

#[test]
fn finite_damage_must_not_overflow_to_nonfinite_persistent_state() {
    let mut e = nous_veritas::Engine::default();
    for _ in 0..2 {
        e.apply("OnDamage",json!({"attacker":{"uid":1002,"team":"Player"},"damage":1e308,"overkill_damage":0,"type":"Normal"})).unwrap();
    }
    let encoded = serde_json::to_vec(&e.snapshot()).unwrap();
    assert!(
        serde_json::from_slice::<nous_veritas::Snapshot>(&encoded).is_ok(),
        "finite inputs summed to infinity; JSON null cannot be read back as f64"
    );
}

#[test]
fn csv_export_must_not_emit_formula_as_untrusted_character_name() {
    let mut b = nous_veritas::Battle::default();
    b.avatars.push(nous_veritas::Combatant {
        name: "=1+1".into(),
        ..Default::default()
    });
    assert!(
        !b.export_csv()
            .lines()
            .nth(1)
            .unwrap()
            .starts_with("character_summary,=1+1,"),
        "CSV formula is exported without neutralization"
    );
}

#[test]
fn normal_veritas_event_roundtrip_control() {
    let mut e = nous_veritas::Engine::default();
    e.apply("OnDamage",json!({"attacker":{"uid":1002,"team":"Player"},"damage":100,"overkill_damage":0,"type":"Normal"})).unwrap();
    let encoded = serde_json::to_vec(&e.snapshot()).unwrap();
    assert!(serde_json::from_slice::<nous_veritas::Snapshot>(&encoded).is_ok());
}

#[test]
fn changed_snapshot_content_must_advance_revision_even_when_movement_is_throttled() {
    use nous_core::world::{Position, WorldCommand};
    let mut g = GameState::new(FileStore::new(None)).unwrap();
    let position = |x| Position {
        x,
        y: 0,
        z: 0,
        rot_y: 0,
        layer: None,
    };
    g.handle(&Command {
        world: Some(WorldCommand::Move(vec![(1, position(1))])),
        ..Default::default()
    })
    .unwrap();
    let before = g.revision();
    g.handle(&Command {
        world: Some(WorldCommand::Move(vec![(1, position(2))])),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(g.account.world.player.unwrap().x, 2);
    assert!(
        g.revision() > before,
        "snapshot content changed while its published revision stayed the same"
    );
}
