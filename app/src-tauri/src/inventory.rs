//! Account inventory as the game sends it: relics, light cones, materials and
//! characters. Values are kept raw (game ids, affix rolls), close to the
//! protocol, so the format doesn't depend on game data tables; names and stat
//! values are resolved when shown. Modelled on reliquary-archiver's exporter.
//!
//! Full lists arrive at login (`GetBagScRsp`, `GetAvatarDataScRsp`), changes
//! afterwards (`PlayerSyncScNotify`).

use std::collections::BTreeMap;

use reliquary::network::command::proto;
use serde::{Deserialize, Serialize};

/// One substat (or pending reroll substat): `cnt` rolls of `affix_id`, `step`
/// extra value steps. Value = cnt * BaseValue + step * StepValue
/// (RelicSubAffixConfig).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Affix {
    pub affix_id: u32,
    pub cnt: u32,
    pub step: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    fn from_proto(p: &proto::Relic::Relic, now: i64) -> Self {
        Self {
            uid: p.unique_id,
            tid: p.tid,
            level: p.level,
            exp: p.exp,
            main_affix_id: p.main_affix_id,
            sub_affixes: affixes(&p.sub_affix_list),
            reforge_sub_affixes: affixes(&p.reforge_sub_affix_list),
            preview_sub_affixes: affixes(&p.preview_sub_affix_list),
            equipped_by: nonzero(p.equip_avatar_id),
            locked: p.is_protected,
            discarded: p.is_discarded,
            first_seen: now,
            updated_at: now,
        }
    }

    /// Same game state, ignoring Nous's own timestamps.
    fn same_as(&self, other: &Relic) -> bool {
        Relic { first_seen: 0, updated_at: 0, ..self.clone() } == Relic { first_seen: 0, updated_at: 0, ..other.clone() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

impl LightCone {
    fn from_proto(p: &proto::Equipment::Equipment) -> Self {
        Self {
            uid: p.unique_id,
            tid: p.tid,
            level: p.level,
            exp: p.exp,
            ascension: p.promotion,
            superimposition: p.rank,
            equipped_by: nonzero(p.equip_avatar_id),
            locked: p.is_protected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    /// ItemConfig id.
    pub tid: u32,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<u64>,
}

/// Level progress shared by all paths of a character (`Avatar`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

impl AvatarPath {
    fn from_proto(p: &proto::AvatarPathData::AvatarPathData) -> Self {
        Self {
            avatar_id: p.avatar_id,
            eidolon: p.rank,
            skill_tree: p.avatar_path_skill_tree.iter().map(|s| (s.point_id, s.level)).collect(),
            skill_tree_version: p.skilltree_version,
            light_cone: nonzero(p.path_equipment_id),
            relics: p.equip_relic_list.iter().map(|r| (r.type_, r.relic_unique_id)).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub relics: BTreeMap<u32, Relic>,
    pub light_cones: BTreeMap<u32, LightCone>,
    pub materials: BTreeMap<u32, Material>,
    pub avatars: BTreeMap<u32, Avatar>,
    pub avatar_paths: BTreeMap<u32, AvatarPath>,
    /// Unix seconds of the last full list from the game (login).
    pub synced_at: Option<i64>,
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
        self.relics.is_empty() && self.deleted_relics.is_empty() && !self.light_cones && !self.materials && !self.characters
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
    pub fn apply_bag(&mut self, bag: &proto::GetBagScRsp::GetBagScRsp, now: i64) -> SnapshotReport {
        let mut report = SnapshotReport::default();
        let mut relics = BTreeMap::new();
        for p in &bag.relic_list {
            let mut relic = Relic::from_proto(p, now);
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
        report.gone = self.relics.keys().filter(|uid| !relics.contains_key(uid)).count();
        self.relics = relics;

        self.light_cones = bag.equipment_list.iter().map(|p| (p.unique_id, LightCone::from_proto(p))).collect();
        self.materials = bag.material_list.iter().map(|m| (m.tid, material(m))).collect();
        self.synced_at = Some(now);
        report
    }

    /// `GetAvatarDataScRsp`: all characters.
    pub fn apply_avatars(&mut self, data: &proto::GetAvatarDataScRsp::GetAvatarDataScRsp) {
        if data.is_get_all {
            self.avatars.clear();
            self.avatar_paths.clear();
        }
        self.merge_avatars(&data.avatar_list, &data.avatar_path_data_info_list);
    }

    /// `PlayerSyncScNotify`: changed and deleted items.
    pub fn apply_sync(&mut self, sync: &proto::PlayerSyncScNotify::PlayerSyncScNotify, now: i64) -> InventoryChanges {
        let mut ch = InventoryChanges::default();

        for p in &sync.relic_list {
            let mut relic = Relic::from_proto(p, now);
            if let Some(old) = self.relics.get(&relic.uid) {
                if relic.same_as(old) {
                    continue;
                }
                relic.first_seen = old.first_seen;
            }
            self.relics.insert(relic.uid, relic.clone());
            ch.relics.push(relic);
        }
        for uid in &sync.del_relic_list {
            if self.relics.remove(uid).is_some() {
                ch.deleted_relics.push(*uid);
            }
        }

        for p in &sync.equipment_list {
            self.light_cones.insert(p.unique_id, LightCone::from_proto(p));
            ch.light_cones = true;
        }
        for uid in &sync.del_equipment_list {
            ch.light_cones |= self.light_cones.remove(uid).is_some();
        }

        for m in &sync.material_list {
            self.materials.insert(m.tid, material(m));
            ch.materials = true;
        }

        if let Some(avatars) = sync.avatar_sync.as_ref() {
            ch.characters = !avatars.avatar_list.is_empty() || !avatars.avatar_path_data_info_list.is_empty();
            self.merge_avatars(&avatars.avatar_list, &avatars.avatar_path_data_info_list);
        }
        ch
    }

    fn merge_avatars(&mut self, avatars: &[proto::Avatar::Avatar], paths: &[proto::AvatarPathData::AvatarPathData]) {
        for a in avatars {
            self.avatars.insert(a.base_avatar_id, Avatar {
                base_avatar_id: a.base_avatar_id,
                level: a.level,
                exp: a.exp,
                ascension: a.promotion,
                current_path: a.cur_multi_path_avatar_type,
            });
        }
        for p in paths {
            self.avatar_paths.insert(p.avatar_id, AvatarPath::from_proto(p));
        }
    }
}

fn affixes(list: &[proto::RelicAffix::RelicAffix]) -> Vec<Affix> {
    list.iter().map(|a| Affix { affix_id: a.affix_id, cnt: a.cnt, step: a.step }).collect()
}

fn material(m: &proto::Material::Material) -> Material {
    Material { tid: m.tid, count: m.num, expire_time: (m.expire_time != 0).then_some(m.expire_time) }
}

fn nonzero(v: u32) -> Option<u32> {
    (v != 0).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn affix(affix_id: u32, cnt: u32, step: u32) -> proto::RelicAffix::RelicAffix {
        let mut a = proto::RelicAffix::RelicAffix::new();
        a.affix_id = affix_id;
        a.cnt = cnt;
        a.step = step;
        a
    }

    fn relic(uid: u32, level: u32, subs: &[(u32, u32, u32)]) -> proto::Relic::Relic {
        let mut r = proto::Relic::Relic::new();
        r.unique_id = uid;
        r.tid = 61011;
        r.main_affix_id = 1;
        r.level = level;
        r.sub_affix_list = subs.iter().map(|&(a, c, s)| affix(a, c, s)).collect();
        r
    }

    fn bag(relics: Vec<proto::Relic::Relic>) -> proto::GetBagScRsp::GetBagScRsp {
        let mut b = proto::GetBagScRsp::GetBagScRsp::new();
        b.relic_list = relics;
        b
    }

    fn sync(relics: Vec<proto::Relic::Relic>, deleted: Vec<u32>) -> proto::PlayerSyncScNotify::PlayerSyncScNotify {
        let mut s = proto::PlayerSyncScNotify::PlayerSyncScNotify::new();
        s.relic_list = relics;
        s.del_relic_list = deleted;
        s
    }

    #[test]
    fn login_list_replaces_and_reports() {
        let mut inv = Inventory::default();
        let first = inv.apply_bag(&bag(vec![relic(1, 0, &[(4, 1, 0)]), relic(2, 0, &[(5, 1, 0)])]), 100);
        assert_eq!(first, SnapshotReport { new: 2, ..Default::default() });

        // next login: relic 1 untouched, 2 levelled meanwhile, 3 salvaged... and new 4
        let second = inv.apply_bag(&bag(vec![relic(1, 0, &[(4, 1, 0)]), relic(2, 3, &[(5, 2, 1)]), relic(4, 0, &[])]), 200);
        assert_eq!(second, SnapshotReport { kept: 1, changed: 1, new: 1, ..Default::default() });
        assert_eq!(inv.relics[&1].updated_at, 100, "unchanged relic keeps its timestamp");
        assert_eq!(inv.relics[&2].first_seen, 100, "first_seen survives changes");
        assert_eq!(inv.relics[&2].updated_at, 200);
    }

    #[test]
    fn reassigned_id_is_reported() {
        let mut inv = Inventory::default();
        inv.apply_bag(&bag(vec![relic(1, 0, &[])]), 100);
        let mut other = relic(1, 0, &[]);
        other.tid = 63045;
        let report = inv.apply_bag(&bag(vec![other]), 200);
        assert_eq!(report.reassigned, 1);
        assert_eq!(inv.relics[&1].first_seen, 200, "a different relic is a new one");
    }

    #[test]
    fn level_up_and_reroll_update_in_place() {
        let mut inv = Inventory::default();
        inv.apply_bag(&bag(vec![relic(7, 12, &[(4, 2, 1), (5, 1, 0)])]), 100);

        let ch = inv.apply_sync(&sync(vec![relic(7, 15, &[(4, 3, 1), (5, 1, 0)])], vec![]), 200);
        assert_eq!(ch.relics.len(), 1);
        assert_eq!(inv.relics[&7].level, 15);
        assert_eq!(inv.relics[&7].first_seen, 100);

        // reroll: the new rolls wait in reforge_sub_affix_list until confirmed
        let mut pending = relic(7, 15, &[(4, 3, 1), (5, 1, 0)]);
        pending.reforge_sub_affix_list = vec![affix(4, 1, 0), affix(5, 3, 2)];
        inv.apply_sync(&sync(vec![pending], vec![]), 300);
        assert_eq!(inv.relics[&7].reforge_sub_affixes.len(), 2);

        let kept = relic(7, 15, &[(4, 1, 0), (5, 3, 2)]);
        inv.apply_sync(&sync(vec![kept], vec![]), 400);
        let r = &inv.relics[&7];
        assert!(r.reforge_sub_affixes.is_empty());
        assert_eq!(r.sub_affixes[1], Affix { affix_id: 5, cnt: 3, step: 2 });
        assert_eq!(r.first_seen, 100);
    }

    #[test]
    fn identical_sync_is_not_a_change_and_delete_removes() {
        let mut inv = Inventory::default();
        inv.apply_bag(&bag(vec![relic(1, 0, &[]), relic(2, 0, &[])]), 100);
        let ch = inv.apply_sync(&sync(vec![relic(1, 0, &[])], vec![2]), 200);
        assert!(ch.relics.is_empty());
        assert_eq!(ch.deleted_relics, vec![2]);
        assert_eq!(inv.relics.len(), 1);
        assert_eq!(inv.relics[&1].updated_at, 100);
    }
}
