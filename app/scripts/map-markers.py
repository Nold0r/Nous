"""Chests and puzzles on the map, for each floor in modules/map/maps.json,
from the datamine.

Puzzles are what the game marks with its puzzle icon (MiniMapIcon 89,
IconMapPlay6), one per group. The icon may sit on the mechanism (the Golden
Spear: where the puzzle starts) and on a placeholder where the reward
appears (where it ends); both are kept. The group's only chest is the puzzle's
reward: an opened one is a solved puzzle, and it is not drawn apart. The
icon also has rules per prop state; a state without it is solved too.
Puzzle reward chests in groups with no icon stand for their puzzle
(older regions).

Nymphs (Amphoreus bugs) are the game's "unknown specimen" icon
(MiniMapIcon 276) over a bug in the same group; the bug's PropState tells
if it was caught (EventOpen, or Closed for some butterflies).

    python scripts/map-markers.py <TurnBasedGameData clone>

Run after map-textures.py (it fills maps.json, this adds `chests` to each
floor). Docs: docs/Архитектура/Модуль карты.md.
"""
import argparse
import glob
import json
import math
import os
import re

OUT_JSON = os.path.join(os.path.dirname(__file__), '..', 'modules', 'map', 'maps.json')
PUZZLE_ICON = 89
NYMPH_ICON = 276
# bug PropStates of a caught nymph
NYMPH_CAUGHT = [19, 0]
# a mechanism this far from its reward gets its own mark
START_APART_M = 3
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
    ["group:id", x, z, layer, PropStates without the icon, reward chest id or 0,
    start [x, z, layer] or None] and nymphs ["group:id" of the bug, x, z, layer,
    caught PropStates]. The puzzle's own point is where it ends
    (the reward), the start is the mechanism when it stands apart."""
    layer = {}
    baked = f'{tb}/Config/LevelOutput_Baked/Floor/P{plane}_F{floor}_Baked.json'
    if os.path.exists(baked):
        for dim in json.load(open(baked, encoding='utf-8'))['DimensionList']:
            for g in dim['GroupList']:
                for o in g.get('PropList') or []:
                    lid = (o.get('NavMap') or {}).get('MapLayerID') or 0
                    if lid > 0:
                        layer[(g['ID'], o['ID'])] = lid
    chests, group_chests, nymphs = {}, {}, {}
    bugs = {}  # group -> "group:id" of its bug
    icons = {}  # group -> {'mechanism': [...], 'placeholder': [...]}
    for f in sorted(glob.glob(f'{tb}/Config/LevelOutput/SharedRuntimeGroup/Groups_P{plane}_F{floor}/*.json')):
        gid = int(re.search(r'_G(\d+)(?:_D\d+)?\.json$', f)[1])
        for o in json.load(open(f, encoding='utf-8')).get('PropList') or []:
            if o.get('IsDelete') or 'PosX' not in o:
                continue
            at = [round(o['PosX'], 1), round(o.get('PosZ', 0.0), 1), layer.get((gid, o.get('ID')), 0)]
            prop = props.get(o.get('PropID'), {})
            rules = o.get('OverrideMapStateIcons') or prop.get('MiniMapStateIcons') or []
            if any(r.get('IconID') == NYMPH_ICON for r in rules):
                nymphs.setdefault(gid, at)
                continue
            if '_Bug' in (prop.get('JsonPath') or ''):
                bugs.setdefault(gid, f'{gid}:{o["ID"]}')
            hide = icon_states(rules, prop.get('PropStateList') or [])
            cid = o.get('ChestID')
            if cid:
                # a chest the game marks as a puzzle (Golden Scapegoat masks) is one
                kind = 'puzzle' if hide is not None else kind_of(types.get(cid, []))
                chests.setdefault(cid, [cid, *at, kind])
                group_chests.setdefault(gid, set()).add(cid)
                continue
            if hide is not None:
                role = 'placeholder' if prop.get('PropType') == 'PROP_ORDINARY' else 'mechanism'
                icons.setdefault(gid, {}).setdefault(role, [f'{gid}:{o["ID"]}', at, hide])
    puzzles = {}
    for gid, roles in icons.items():
        mech, place = roles.get('mechanism'), roles.get('placeholder')
        # the mechanism's state tells solved (the spear is Locked when broken)
        key, _, hide = mech or place
        end = (place or mech)[1]
        start = mech[1] if mech and place and math.dist(mech[1][:2], place[1][:2]) > START_APART_M else None
        puzzles[gid] = [key, *end, hide, 0, start]
    for gid, p in puzzles.items():
        own = group_chests.get(gid, set())
        if len(own) == 1:
            p[5] = chests.pop(next(iter(own)))[0]
    nymph_rows = [[bugs.get(gid, ''), *at, NYMPH_CAUGHT] for gid, at in nymphs.items()]
    return list(chests.values()), list(puzzles.values()), nymph_rows


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
    total = icons = bugs = 0
    for floor, m in maps.items():
        m['chests'], m['puzzles'], m['nymphs'] = floor_marks(tb, planes[floor], int(floor), types, props)
        bugs += len(m['nymphs'])
        total += len(m['chests'])
        icons += len(m['puzzles'])
    with open(OUT_JSON, 'w', encoding='utf-8') as fp:
        json.dump(maps, fp, ensure_ascii=False, separators=(',', ':'))
    print(f'{len(maps)} floors, {total} chests, {icons} puzzle icons, {bugs} nymphs')


if __name__ == '__main__':
    main()
