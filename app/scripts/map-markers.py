"""Chests on the map: every MazeChest of each floor in modules/map/maps.json
with its world position, map layer and kind, from the datamine.

    python scripts/map-markers.py <TurnBasedGameData clone>

Run after map-textures.py (it fills maps.json, this adds `chests` to each
floor). Docs: docs/Архитектура/Модуль карты.md.
"""
import argparse
import glob
import json
import os
import re

OUT_JSON = os.path.join(os.path.dirname(__file__), '..', 'modules', 'map', 'maps.json')
LISTS = ('PropList', 'MonsterList', 'AnchorList', 'NPCList')


def kind_of(types):
    """'puzzle' for puzzle rewards, else the chest's tier."""
    if any('PUZZLE' in t for t in types):
        return 'puzzle'
    if 'CHEST_HIGH_LEVEL' in types:
        return 'precious'
    if 'CHEST_MIDDLE_LEVEL' in types:
        return 'bountiful'
    return 'basic'


def floor_chests(tb, plane, floor, types):
    """[chest id, x, z, map layer (0 unknown), kind] of the floor's chests."""
    layer = {}
    baked = f'{tb}/Config/LevelOutput_Baked/Floor/P{plane}_F{floor}_Baked.json'
    if os.path.exists(baked):
        for dim in json.load(open(baked, encoding='utf-8'))['DimensionList']:
            for g in dim['GroupList']:
                for o in g.get('PropList') or []:
                    lid = (o.get('NavMap') or {}).get('MapLayerID') or 0
                    if lid > 0:
                        layer[(g['ID'], o['ID'])] = lid
    out = {}
    for f in sorted(glob.glob(f'{tb}/Config/LevelOutput/SharedRuntimeGroup/Groups_P{plane}_F{floor}/*.json')):
        gid = int(re.search(r'_G(\d+)(?:_D\d+)?\.json$', f)[1])
        for o in json.load(open(f, encoding='utf-8')).get('PropList') or []:
            cid = o.get('ChestID')
            if not cid or o.get('IsDelete') or 'PosX' not in o or cid in out:
                continue
            out[cid] = [cid, round(o['PosX'], 1), round(o.get('PosZ', 0.0), 1),
                        layer.get((gid, o.get('ID')), 0), kind_of(types.get(cid, []))]
    return list(out.values())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('tbgd')
    tb = ap.parse_args().tbgd
    types = {c['ID']: c.get('ChestType') or [] for c in json.load(open(f'{tb}/ExcelOutput/MazeChest.json', encoding='utf-8'))}
    planes = {}
    for fn in os.listdir(f'{tb}/Config/LevelOutput/Map'):
        m = re.match(r'MapInfo_P(\d+)_F(\d+)\.json', fn)
        planes[m[2]] = int(m[1])
    maps = json.load(open(OUT_JSON, encoding='utf-8'))
    total = 0
    for floor, m in maps.items():
        m['chests'] = floor_chests(tb, planes[floor], int(floor), types)
        total += len(m['chests'])
    with open(OUT_JSON, 'w', encoding='utf-8') as fp:
        json.dump(maps, fp, ensure_ascii=False, separators=(',', ':'))
    print(f'{len(maps)} floors, {total} chests')


if __name__ == '__main__':
    main()
