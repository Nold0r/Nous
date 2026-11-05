//! Account inventory as the game sends it: relics, light cones, materials and
//! characters. Values are kept raw (game ids, affix rolls), close to the
//! protocol, so the format doesn't depend on game data tables; names and stat
//! values are resolved when shown. Modelled on reliquary-archiver's exporter.
//!
//! Full lists arrive at login (`GetBagScRsp`, `GetAvatarDataScRsp`), changes
//! afterwards (`PlayerSyncScNotify`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One substat (or pending reroll substat): `cnt` rolls of `affix_id`, `step`
/// extra value steps. Value = cnt * BaseValue + step * StepValue
/// (RelicSubAffixConfig).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Affix {
    pub affix_id: u32,
    pub cnt: u32,
    pub step: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Relic {
    /// `unique_id` from the game: the relic's identity in this account.
    pub uid: u32,
    /// RelicConfig id: set, slot and rarity.
    pub tid: u32,
    pub level: u32,
    pub exp: u32,
    /// RelicMainAffixConfig id within the relic's MainAffixGroup.
    pub main_affix_id: u32,
    pub sub_affixes: Vec<Affix>,
    /// Reroll result waiting for the player to keep or discard it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reforge_sub_affixes: Vec<Affix>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preview_sub_affixes: Vec<Affix>,
    /// Character (path avatar id) wearing it.
    pub equipped_by: Option<u32>,
    pub locked: bool,
    pub discarded: bool,
    /// Unix seconds when Nous first saw this relic; kept across updates.
    pub first_seen: i64,
    /// Unix seconds of the last change Nous saw.
    pub updated_at: i64,
}

impl Relic {
    /// Same game state, ignoring Nous's own timestamps.
    fn same_as(&self, other: &Relic) -> bool {
        Relic {
            first_seen: 0,
            updated_at: 0,
            ..self.clone()
        } == Relic {
            first_seen: 0,
            updated_at: 0,
            ..other.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct LightCone {
    pub uid: u32,
    /// EquipmentConfig id.
    pub tid: u32,
    pub level: u32,
    pub exp: u32,
    pub ascension: u32,
    pub superimposition: u32,
    pub equipped_by: Option<u32>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Material {
    /// ItemConfig id.
    pub tid: u32,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<u64>,
}

/// Level progress shared by all paths of a character (`Avatar`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct Avatar {
    pub base_avatar_id: u32,
    pub level: u32,
    pub exp: u32,
    pub ascension: u32,
    /// Path currently selected (multi-path characters), 0 otherwise.
    pub current_path: u32,
}

/// Per-path build (`AvatarPathData`): eidolons, traces, gear. For ordinary
/// characters `avatar_id` equals the base id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
// missing fields (older files) take defaults: adding a field never breaks loading
#[serde(rename_all = "camelCase", default)]
pub struct AvatarPath {
    pub avatar_id: u32,
    pub eidolon: u32,
    /// AvatarSkillTreeConfig point id -> level.
    pub skill_tree: BTreeMap<u32, u32>,
    pub skill_tree_version: u32,
    /// Light cone uid.
    pub light_cone: Option<u32>,
    /// Relic slot (1 head … 6 rope) -> relic uid.
    pub relics: BTreeMap<u32, u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Inventory {
    pub relics: BTreeMap<u32, Relic>,
    pub light_cones: BTreeMap<u32, LightCone>,
    pub materials: BTreeMap<u32, Material>,
    pub avatars: BTreeMap<u32, Avatar>,
    pub avatar_paths: BTreeMap<u32, AvatarPath>,
    /// Unix seconds of the last full list from the game (login).
    pub synced_at: Option<i64>,
    /// Unix seconds of the first full list Nous saw for this account. Relics
    /// first seen then were already owned, not new drops (UI "new" marks).
    /// None in files from before it was added: their relics are all older.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported_at: Option<i64>,
}

/// What a command changed; sent to the UI as deltas.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryChanges {
    pub relics: Vec<Relic>,
    pub deleted_relics: Vec<u32>,
    pub light_cones: bool,
    pub materials: bool,
    pub characters: bool,
}

impl InventoryChanges {
    pub fn is_empty(&self) -> bool {
        self.relics.is_empty()
            && self.deleted_relics.is_empty()
            && !self.light_cones
            && !self.materials
            && !self.characters
    }
}

/// How a fresh login list compares to what was stored: tells whether relic
/// ids survive between sessions.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotReport {
    /// Same uid, same relic (tid and main stat).
    pub kept: usize,
    /// Same uid but a different relic: the id was reused or reassigned.
    pub reassigned: usize,
    /// Same uid and relic, rolls or level changed while Nous wasn't watching.
    pub changed: usize,
    pub new: usize,
    pub gone: usize,
}

impl Inventory {
    /// `GetBagScRsp`: full relic, light cone and material lists.
    pub fn apply_bag(&mut self, bag: &Bag, now: i64) -> SnapshotReport {
        let mut report = SnapshotReport::default();
        let mut relics = BTreeMap::new();
        for p in &bag.relics {
            let mut relic = Relic {
                first_seen: now,
                updated_at: now,
                ..p.clone()
            };
            match self.relics.get(&relic.uid) {
                Some(old) if old.tid == relic.tid && old.main_affix_id == relic.main_affix_id => {
                    relic.first_seen = old.first_seen;
                    if relic.same_as(old) {
                        relic.updated_at = old.updated_at;
                        report.kept += 1;
                    } else {
                        report.changed += 1;
                    }
                }
                Some(_) => report.reassigned += 1,
                None => report.new += 1,
            }
            relics.insert(relic.uid, relic);
        }
        report.gone = self
            .relics
            .keys()
            .filter(|uid| !relics.contains_key(uid))
            .count();
        self.relics = relics;

        self.light_cones = bag.light_cones.iter().map(|p| (p.uid, p.clone())).collect();
        self.materials = bag.materials.iter().map(|m| (m.tid, m.clone())).collect();
        if self.synced_at.is_none() {
            self.imported_at = Some(now);
        }
        self.synced_at = Some(now);
        report
    }

    /// `GetAvatarDataScRsp`: all characters.
    pub fn apply_avatars(&mut self, data: &Characters) {
        if data.replace {
            self.avatars.clear();
            self.avatar_paths.clear();
        }
        self.merge_avatars(&data.avatars, &data.paths);
    }

    /// `PlayerSyncScNotify`: changed and deleted items.
    pub fn apply_sync(&mut self, sync: &InventorySync, now: i64) -> InventoryChanges {
        let mut ch = InventoryChanges::default();

        for p in &sync.relics {
            let mut relic = Relic {
                first_seen: now,
                updated_at: now,
                ..p.clone()
            };
            if let Some(old) = self.relics.get(&relic.uid) {
                if relic.same_as(old) {
                    continue;
                }
                relic.first_seen = old.first_seen;
            }
            self.relics.insert(relic.uid, relic.clone());
            ch.relics.push(relic);
        }
        for uid in &sync.deleted_relics {
            if self.relics.remove(uid).is_some() {
                ch.deleted_relics.push(*uid);
            }
        }

        for p in &sync.light_cones {
            self.light_cones.insert(p.uid, p.clone());
            ch.light_cones = true;
        }
        for uid in &sync.deleted_light_cones {
            ch.light_cones |= self.light_cones.remove(uid).is_some();
        }

        for m in &sync.materials {
            self.materials.insert(m.tid, m.clone());
            ch.materials = true;
        }

        if let Some(avatars) = sync.characters.as_ref() {
            ch.characters = !avatars.avatars.is_empty() || !avatars.paths.is_empty();
            self.merge_avatars(&avatars.avatars, &avatars.paths);
        }
        ch
    }

    fn merge_avatars(&mut self, avatars: &[Avatar], paths: &[AvatarPath]) {
        self.avatars
            .extend(avatars.iter().map(|a| (a.base_avatar_id, a.clone())));
        self.avatar_paths
            .extend(paths.iter().map(|p| (p.avatar_id, p.clone())));
    }
}

#[derive(Debug, Clone, Default)]
pub struct Bag {
    pub relics: Vec<Relic>,
    pub light_cones: Vec<LightCone>,
    pub materials: Vec<Material>,
}
#[derive(Debug, Clone, Default)]
pub struct Characters {
    pub replace: bool,
    pub avatars: Vec<Avatar>,
    pub paths: Vec<AvatarPath>,
}
#[derive(Debug, Clone, Default)]
pub struct InventorySync {
    pub relics: Vec<Relic>,
    pub deleted_relics: Vec<u32>,
    pub light_cones: Vec<LightCone>,
    pub deleted_light_cones: Vec<u32>,
    pub materials: Vec<Material>,
    pub characters: Option<Characters>,
}
