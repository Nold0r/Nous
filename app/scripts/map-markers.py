"""Chests and puzzles on the map, for each floor in modules/map/maps.json,
from the datamine.

Puzzles are what the game marks with its puzzle icon (MiniMapIcon 89,
IconMapPlay6), one per group. The group's only chest is the puzzle's
reward: an opened one is a solved puzzle, and it is not drawn apart. The
icon also has rules per prop state; a state without it is solved too.
Puzzle reward chests in groups with no icon stand for their puzzle
(older regions).

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
PUZZLE_ICON = 89
# the protocol's PropState numbers
PROP_STATES = {
    'Closed': 0, 'Open': 1, 'Locked': 2, 'BridgeState1': 3, 'BridgeState2': 4, 'BridgeState3': 5,
    'BridgeState4': 6, 'CheckPointDisable': 7, 'CheckPointEnable': 8, 'TriggerDisable': 9,
    'TriggerEnable': 10, 'ChestLocked': 11, 'ChestClosed': 12, 'ChestUsed': 13, 'Elevator1': 14,
    'Elevator2': 15, 'Elevator3': 16, 'WaitActive': 17, 'EventClose': 18, 'EventOpen': 19,
    'Hidden': 20, 'TeleportGate0': 21, 'TeleportGate1': 22, 'TeleportGate2': 23, 'TeleportGate3': 24,
    'Destructed': 25, **{f'CustomState0{i}': 100 + i for i in range(1, 10)},
}


def kind_of(types):
    """'puzzle' for puzzle rewards, else the chest's tier."""
    if any('PUZZLE' in t for t in types):
        return 'puzzle'
    if 'CHEST_HIGH_LEVEL' in types:
        return 'precious'
    if 'CHEST_MIDDLE_LEVEL' in types:
        return 'bountiful'
    return 'basic'


def icon_states(rules, states):
    """PropState numbers in which the puzzle icon is gone; None if the prop
    never shows it. A rule for the state wins, a rule without a state is
    the default."""
    default = next((r.get('IconID') for r in rules if 'State' not in r), None)
    by_state = {r['State']: r.get('IconID') for r in rules if 'State' in r}
    shown = {s: by_state.get(s, default) == PUZZLE_ICON for s in set(states) | set(by_state)}
    if not any(shown.values()) and default != PUZZLE_ICON:
        return None
    return sorted(PROP_STATES[s] for s, on in shown.items() if not on and s in PROP_STATES)


def floor_marks(tb, plane, floor, types, props):
    """Chests [chest id, x, z, map layer (0 unknown), kind] and puzzles
    ["group:id", x, z, layer, PropStates without the icon, reward chest id or 0]."""
    layer = {}
    baked = f'{tb}/Config/LevelOutput_Baked/Floor/P{plane}_F{floor}_Baked.json'
    if os.path.exists(baked):
        for dim in json.load(open(baked, encoding='utf-8'))['DimensionList']:
            for g in dim['GroupList']:
                for o in g.get('PropList') or []:
                    lid = (o.get('NavMap') or {}).get('MapLayerID') or 0
                    if lid > 0:
                        layer[(g['ID'], o['ID'])] = lid
    chests, puzzles, group_chests = {}, {}, {}
    for f in sorted(glob.glob(f'{tb}/Config/LevelOutput/SharedRuntimeGroup/Groups_P{plane}_F{floor}/*.json')):
        gid = int(re.search(r'_G(\d+)(?:_D\d+)?\.json$', f)[1])
        for o in json.load(open(f, encoding='utf-8')).get('PropList') or []:
            if o.get('IsDelete') or 'PosX' not in o:
                continue
            at = [round(o['PosX'], 1), round(o.get('PosZ', 0.0), 1), layer.get((gid, o.get('ID')), 0)]
            prop = props.get(o.get('PropID'), {})
            rules = o.get('OverrideMapStateIcons') or prop.get('MiniMapStateIcons') or []
            hide = icon_states(rules, prop.get('PropStateList') or [])
            cid = o.get('ChestID')
            if cid:
                # a chest the game marks as a puzzle (Golden Scapegoat masks) is one
                kind = 'puzzle' if hide is not None else kind_of(types.get(cid, []))
                chests.setdefault(cid, [cid, *at, kind])
                group_chests.setdefault(gid, set()).add(cid)
                continue
            # one per group; the placeholder icon over a mechanism wins
            if hide is not None and (gid not in puzzles or prop.get('PropType') == 'PROP_ORDINARY'):
                puzzles[gid] = [f'{gid}:{o["ID"]}', *at, hide, 0]
    for gid, p in puzzles.items():
        own = group_chests.get(gid, set())
        if len(own) == 1:
            p[5] = chests.pop(next(iter(own)))[0]
    return list(chests.values()), list(puzzles.values())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('tbgd')
    tb = ap.parse_args().tbgd
    types = {c['ID']: c.get('ChestType') or [] for c in json.load(open(f'{tb}/ExcelOutput/MazeChest.json', encoding='utf-8'))}
    props = {p['ID']: p for p in json.load(open(f'{tb}/ExcelOutput/MazeProp.json', encoding='utf-8'))}
    planes = {}
    for fn in os.listdir(f'{tb}/Config/LevelOutput/Map'):
        m = re.match(r'MapInfo_P(\d+)_F(\d+)\.json', fn)
        planes[m[2]] = int(m[1])
    maps = json.load(open(OUT_JSON, encoding='utf-8'))
    total = icons = 0
    for floor, m in maps.items():
        m['chests'], m['puzzles'] = floor_marks(tb, planes[floor], int(floor), types, props)
        total += len(m['chests'])
        icons += len(m['puzzles'])
    with open(OUT_JSON, 'w', encoding='utf-8') as fp:
        json.dump(maps, fp, ensure_ascii=False, separators=(',', ':'))
    print(f'{len(maps)} floors, {total} chests, {icons} puzzle icons')


if __name__ == '__main__':
    main()
