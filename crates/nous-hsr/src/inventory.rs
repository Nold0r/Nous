use nous_core::inventory::*;
use reliquary::network::command::proto;
trait RelicProto {
    fn from_proto(p: &proto::Relic::Relic, now: i64) -> Self;
}
impl RelicProto for Relic {
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
}
trait LightConeProto {
    fn from_proto(p: &proto::Equipment::Equipment) -> Self;
}
impl LightConeProto for LightCone {
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
trait AvatarPathProto {
    fn from_proto(p: &proto::AvatarPathData::AvatarPathData) -> Self;
}
impl AvatarPathProto for AvatarPath {
    fn from_proto(p: &proto::AvatarPathData::AvatarPathData) -> Self {
        Self {
            avatar_id: p.avatar_id,
            eidolon: p.rank,
            skill_tree: p
                .avatar_path_skill_tree
                .iter()
                .map(|s| (s.point_id, s.level))
                .collect(),
            skill_tree_version: p.skilltree_version,
            light_cone: nonzero(p.path_equipment_id),
            relics: p
                .equip_relic_list
                .iter()
                .map(|r| (r.type_, r.relic_unique_id))
                .collect(),
        }
    }
}
fn affixes(list: &[proto::RelicAffix::RelicAffix]) -> Vec<Affix> {
    list.iter()
        .map(|a| Affix {
            affix_id: a.affix_id,
            cnt: a.cnt,
            step: a.step,
        })
        .collect()
}

fn material(m: &proto::Material::Material) -> Material {
    Material {
        tid: m.tid,
        count: m.num,
        expire_time: (m.expire_time != 0).then_some(m.expire_time),
    }
}

fn nonzero(v: u32) -> Option<u32> {
    (v != 0).then_some(v)
}

pub fn bag(p: &proto::GetBagScRsp::GetBagScRsp) -> Bag {
    Bag {
        relics: p
            .relic_list
            .iter()
            .map(|r| Relic::from_proto(r, 0))
            .collect(),
        light_cones: p.equipment_list.iter().map(LightCone::from_proto).collect(),
        materials: p.material_list.iter().map(material).collect(),
    }
}
pub fn characters(
    avatars: &[proto::Avatar::Avatar],
    paths: &[proto::AvatarPathData::AvatarPathData],
    replace: bool,
) -> Characters {
    Characters {
        replace,
        avatars: avatars
            .iter()
            .map(|a| Avatar {
                base_avatar_id: a.base_avatar_id,
                level: a.level,
                exp: a.exp,
                ascension: a.promotion,
                current_path: a.cur_multi_path_avatar_type,
            })
            .collect(),
        paths: paths.iter().map(AvatarPath::from_proto).collect(),
    }
}
pub fn sync(p: &proto::PlayerSyncScNotify::PlayerSyncScNotify) -> InventorySync {
    InventorySync {
        relics: p
            .relic_list
            .iter()
            .map(|r| Relic::from_proto(r, 0))
            .collect(),
        deleted_relics: p.del_relic_list.clone(),
        light_cones: p.equipment_list.iter().map(LightCone::from_proto).collect(),
        deleted_light_cones: p.del_equipment_list.clone(),
        materials: p.material_list.iter().map(material).collect(),
        characters: p
            .avatar_sync
            .as_ref()
            .map(|a| characters(&a.avatar_list, &a.avatar_path_data_info_list, false)),
    }
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

    fn bag(relics: Vec<proto::Relic::Relic>) -> Bag {
        let mut b = proto::GetBagScRsp::GetBagScRsp::new();
        b.relic_list = relics;
        super::bag(&b)
    }

    fn sync(relics: Vec<proto::Relic::Relic>, deleted: Vec<u32>) -> InventorySync {
        let mut s = proto::PlayerSyncScNotify::PlayerSyncScNotify::new();
        s.relic_list = relics;
        s.del_relic_list = deleted;
        super::sync(&s)
    }

    #[test]
    fn login_list_replaces_and_reports() {
        let mut inv = Inventory::default();
        let first = inv.apply_bag(
            &bag(vec![relic(1, 0, &[(4, 1, 0)]), relic(2, 0, &[(5, 1, 0)])]),
            100,
        );
        assert_eq!(
            first,
            SnapshotReport {
                new: 2,
                ..Default::default()
            }
        );

        // next login: relic 1 untouched, 2 levelled meanwhile, 3 salvaged... and new 4
        let second = inv.apply_bag(
            &bag(vec![
                relic(1, 0, &[(4, 1, 0)]),
                relic(2, 3, &[(5, 2, 1)]),
                relic(4, 0, &[]),
            ]),
            200,
        );
        assert_eq!(
            second,
            SnapshotReport {
                kept: 1,
                changed: 1,
                new: 1,
                ..Default::default()
            }
        );
        assert_eq!(
            inv.relics[&1].updated_at, 100,
            "unchanged relic keeps its timestamp"
        );
        assert_eq!(
            inv.relics[&2].first_seen, 100,
            "first_seen survives changes"
        );
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
        assert_eq!(
            inv.relics[&1].first_seen, 200,
            "a different relic is a new one"
        );
    }

    #[test]
    fn level_up_and_reroll_update_in_place() {
        let mut inv = Inventory::default();
        inv.apply_bag(&bag(vec![relic(7, 12, &[(4, 2, 1), (5, 1, 0)])]), 100);

        let ch = inv.apply_sync(
            &sync(vec![relic(7, 15, &[(4, 3, 1), (5, 1, 0)])], vec![]),
            200,
        );
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
        assert_eq!(
            r.sub_affixes[1],
            Affix {
                affix_id: 5,
                cnt: 3,
                step: 2
            }
        );
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
