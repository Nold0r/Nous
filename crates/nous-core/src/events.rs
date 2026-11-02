use crate::game::{Changes, GameState, Versioned};
use crate::inventory::InventoryChanges;
use crate::world::WorldChange;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InventoryEvent {
    Reset,
    Changed(InventoryChanges),
}

/// The same event envelope is usable over a channel, IPC or another transport.
#[derive(Debug, Clone)]
pub struct Event {
    pub name: &'static str,
    pub payload: serde_json::Value,
}
impl Event {
    pub fn new(name: &'static str, payload: impl Serialize) -> Self {
        Self {
            name,
            payload: serde_json::to_value(payload).expect("SDK events are JSON serializable"),
        }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(
            "sdk-error",
            serde_json::json!({ "message": message.into() }),
        )
    }
}
pub(crate) fn changes(game: &GameState, ch: Changes) -> Vec<Event> {
    let mut out = Vec::new();
    let mut emit = |name, data| {
        out.push(Event::new(
            name,
            Versioned {
                revision: ch.revision,
                data,
            },
        ))
    };
    if ch.account_switched {
        emit("account-changed", serde_json::json!(game.account.uid));
    }
    if ch.profile {
        emit("profile-changed", serde_json::json!(game.profile()));
    }
    if ch.inventory_reset {
        emit(
            "inventory-changed",
            serde_json::json!(InventoryEvent::Reset),
        );
    } else if !ch.inventory.is_empty() {
        emit(
            "inventory-changed",
            serde_json::json!(InventoryEvent::Changed(ch.inventory)),
        );
    }
    if ch.world == WorldChange::Changed {
        emit("world-changed", serde_json::json!(game.account.world));
    } else if ch.world == WorldChange::Moved {
        emit("player-moved", serde_json::json!(game.account.world.player));
    }
    if let Some(battle) = ch.battle {
        emit("battle-finished", serde_json::json!(battle));
    }
    if ch.gacha {
        emit("gacha-changed", serde_json::Value::Null);
    }
    if let Some(name) = ch.snapshot {
        emit("snapshot-changed", serde_json::json!(name));
    }
    out
}
