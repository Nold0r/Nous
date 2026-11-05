//! The player's own notes, stored in the account file next to the game data.
//! Relic notes are keyed by the relic `uid` (the game's unique_id, stable
//! while the relic exists, see docs: Данные аккаунта). A note outlives its
//! relic: nothing is deleted when the relic is salvaged.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Longest note kept, in characters.
pub const MAX_LEN: usize = 2000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Notes {
    /// relic uid -> note
    pub relics: BTreeMap<u32, Note>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Note {
    pub text: String,
    /// Unix seconds of the last edit.
    pub updated_at: i64,
}

impl Notes {
    /// Sets the note of relic `uid`; blank text removes it. Returns whether anything changed.
    pub fn set_relic(&mut self, uid: u32, text: &str, now: i64) -> bool {
        let text: String = text.trim().chars().take(MAX_LEN).collect();
        if text.is_empty() {
            return self.relics.remove(&uid).is_some();
        }
        if self.relics.get(&uid).is_some_and(|n| n.text == text) {
            return false;
        }
        self.relics.insert(uid, Note { text, updated_at: now });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_trim_and_remove() {
        let mut notes = Notes::default();
        assert!(notes.set_relic(17054, "  на Ахерон  ", 10));
        assert_eq!(notes.relics[&17054].text, "на Ахерон");
        assert!(!notes.set_relic(17054, "на Ахерон", 20), "same text is not a change");
        assert_eq!(notes.relics[&17054].updated_at, 10);
        assert!(notes.set_relic(17054, "   ", 30));
        assert!(notes.relics.is_empty());
        assert!(!notes.set_relic(17054, "", 40), "removing a missing note changes nothing");
    }

    #[test]
    fn long_notes_are_cut() {
        let mut notes = Notes::default();
        notes.set_relic(1, &"я".repeat(MAX_LEN + 10), 0);
        assert_eq!(notes.relics[&1].text.chars().count(), MAX_LEN);
    }
}
