//! Nous SDK core. No desktop runtime, capture backend or protocol dependency.
pub mod account;
pub mod battles;
mod events;
pub mod gacha;
pub mod game;
pub mod inventory;
pub mod notes;
pub mod profile;
mod sdk;
pub mod snapshots;
pub mod world;
pub use account::{Account, AccountStore, FileStore};
pub use events::{Event, InventoryEvent};
pub use game::{Command, GameState, Versioned};
pub use sdk::Client;
/// API compatibility is versioned independently of account files.
pub const API_VERSION: u32 = 1;
