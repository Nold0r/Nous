use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Banner {
    pub gacha_id: u32,
    /// Unix seconds; 0 for permanent banners.
    pub begin: i64,
    pub end: i64,
    /// Featured item ids (rate-up).
    pub featured: Vec<u32>,
    /// 5* item ids of the banner.
    pub prize: Vec<u32>,
}

/// One pull from the official history.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Pull {
    /// Record id (string of digits): unique, grows with time.
    pub id: String,
    pub gacha_type: u32,
    pub gacha_id: u32,
    pub item_id: u32,
    /// 3, 4 or 5 stars.
    pub rank: u32,
    /// "Персонажи" / "Световые конусы" (in the language of the request).
    pub item_type: String,
    pub name: String,
    /// Server local time, "YYYY-MM-DD HH:MM:SS".
    pub time: String,
}

/// A pull seen in the traffic (no record id or time from the server).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct LivePull {
    pub gacha_id: u32,
    pub item_id: u32,
    pub is_new: bool,
    /// Unix seconds Nous saw it.
    pub at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Gacha {
    pub banners: Vec<Banner>,
    /// Official history, record id -> pull.
    pub history: BTreeMap<String, Pull>,
    /// Unix seconds of the last history import.
    pub imported_at: Option<i64>,
    pub live: Vec<LivePull>,
}

impl Gacha {
    /// Adds imported pulls; returns how many were new.
    pub fn merge(&mut self, pulls: Vec<Pull>, now: i64) -> usize {
        let before = self.history.len();
        for p in pulls {
            self.history.insert(p.id.clone(), p);
        }
        self.imported_at = Some(now);
        self.history.len() - before
    }
}
