"""Chests and puzzles on the map, for each floor in modules/map/maps.json,
from the datamine.

Puzzles are what the game marks with its puzzle icon (MiniMapIcon 89 or
402, IconMapPlay6), one per group. The icon may sit on the mechanism (the Golden
Spear: where the puzzle starts) and on a placeholder where the reward
appears (where it ends); both are kept. The group's only chest is the puzzle's
reward: an opened one is a solved puzzle, and it is not drawn apart. The
icon also has rules per prop state; a state without it is solved too.
Puzzle reward chests in groups with no icon stand for their puzzle
(older regions).

Finds are collectibles scattered over the maps, each the game's own map
icon over a creature or thing in the same group whose PropState goes
EventClose -> EventOpen when it is collected (the group templates share one
scheme: Penacony's origami birds, Amphoreus' bugs -- nymphs --, the golden
chenling stickers -- star rabbits). Some butterflies end Closed instead.

Spirithieves are the groups named "QTEGotchard": the spirit (MascotMonster)
and a chest with no MazeChest id. Once caught, the game loads a
"QTEGotchard_Reward" group with a chest on the same spot; that chest is
ChestUsed when taken (the spirit group's own chest stays ChestClosed).

    python scripts/map-markers.py <TurnBasedGameData clone>

Run after map-textures.py (it fills maps.json, this adds `chests` to each
floor). Docs: docs/Архитектура/Модуль карты.md.
"""
import argparse
import collections
import glob
import json
import math
import os
import re

OUT_JSON = os.path.join(os.path.dirname(__file__), '..', 'modules', 'map', 'maps.json')
# the game's puzzle icons (IconMapPlay6; 402 is the same picture on newer maps)
PUZZLE_ICONS = {89, 402}
# map icon -> kind, and what the collected thing's prop config is called
FINDS = {
    276: ('nymph', '_Bug'),  # IconMapSpecimenUnknown
    158: ('origami', 'OrigamiBird'),  # IconMapOrigamiFur
    396: ('rabbit', 'GCLSticker'),  # IconMapSticker
}
# PropStates of a collected find
FOUND = [19, 0]
SPIRITHIEF_GROUP = 'QTEGotchard'
SPIRITHIEF_REWARD = ('QTEGotchard_Reward', 'QTEGotchardReward')
# a reward chest this close to a spirit is its reward
REWARD_REACH_M = 10
CHEST_USED = 13
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
    shown = {s: by_state.get(s, default) in PUZZLE_ICONS for s in set(states) | set(by_state)}
    if not any(shown.values()) and default not in PUZZLE_ICONS:
        return None
    return sorted(PROP_STATES[s] for s, on in shown.items() if not on and s in PROP_STATES)


def floor_marks(tb, plane, floor, types, props):
    """Chests [chest id, x, z, map layer (0 unknown), kind] and puzzles
    ["group:id", x, z, layer, PropStates without the icon, reward chest id or 0,
    start [x, z, layer] or None] and finds [kind, "group:id" of the thing, x, z,
    layer, collected PropStates]. The puzzle's own point is where it ends
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
    chests, group_chests, finds = {}, {}, {}
    things = {}  # (group, kind) -> "group:id" of the thing collected
    spirits, rewards = [], []
    icons = {}  # group -> {'mechanism': [...], 'placeholder': [...]}
    for f in sorted(glob.glob(f'{tb}/Config/LevelOutput/SharedRuntimeGroup/Groups_P{plane}_F{floor}/*.json')):
        gid = int(re.search(r'_G(\d+)(?:_D\d+)?\.json$', f)[1])
        group = json.load(open(f, encoding='utf-8'))
        if group.get('GroupName') == SPIRITHIEF_GROUP:
            spirits.append(spirithief(gid, group, props, layer))
        if group.get('GroupName') in SPIRITHIEF_REWARD:
            rewards += [(f'{gid}:{o["ID"]}', o['PosX'], o.get('PosZ', 0.0), o.get('ChestID'))
                        for o in group.get('PropList') or []
                        if props.get(o.get('PropID'), {}).get('PropType') == 'PROP_TREASURE_CHEST' and 'PosX' in o]
        for o in group.get('PropList') or []:
            if o.get('IsDelete') or 'PosX' not in o:
                continue
            at = [round(o['PosX'], 1), round(o.get('PosZ', 0.0), 1), layer.get((gid, o.get('ID')), 0)]
            prop = props.get(o.get('PropID'), {})
            rules = o.get('OverrideMapStateIcons') or prop.get('MiniMapStateIcons') or []
            find = next((FINDS[r['IconID']][0] for r in rules if r.get('IconID') in FINDS), None)
            if find:
                finds.setdefault((gid, find), at)
                continue
            for kind, config in FINDS.values():
                if config in (prop.get('JsonPath') or ''):
                    things.setdefault((gid, kind), f'{gid}:{o["ID"]}')
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
    find_rows = [[kind, things.get((gid, kind), ''), *at, FOUND] for (gid, kind), at in finds.items()]
    for sp in filter(None, spirits):
        near = min(rewards, key=lambda r: math.dist(r[1:3], sp[1:3]), default=None)
        if near and math.dist(near[1:3], sp[1:3]) <= REWARD_REACH_M:
            sp[0] = near[0]
            # a reward that is a MazeChest tells caught by the opened list, and is not drawn apart
            if near[3] in chests:
                sp[5] = chests.pop(near[3])[0]
    return list(chests.values()), list(puzzles.values()), find_rows, [s for s in spirits if s]


def spirithief(gid, group, props, layer):
    """["group:id" of its chest, x, z, layer, [ChestUsed], MazeChest id or 0]
    at the spirit; the chest is replaced by its reward group's one when there
    is one."""
    spirit = chest = None
    for o in group.get('PropList') or []:
        path = props.get(o.get('PropID'), {}).get('JsonPath') or ''
        if 'MascotMonster' in path and 'PosX' in o:
            spirit = o
        elif props.get(o.get('PropID'), {}).get('PropType') == 'PROP_TREASURE_CHEST':
            chest = o
    if not spirit:
        return None
    at = [round(spirit['PosX'], 1), round(spirit.get('PosZ', 0.0), 1), layer.get((gid, spirit['ID']), 0)]
    return [f'{gid}:{chest["ID"]}' if chest else '', *at, [CHEST_USED], chest.get('ChestID') or 0 if chest else 0]


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
    total = icons = spirits = 0
    found = collections.Counter()
    for floor, m in maps.items():
        m['chests'], m['puzzles'], m['finds'], m['spirithieves'] = floor_marks(tb, planes[floor], int(floor), types, props)
        m.pop('nymphs', None)
        found.update(f[0] for f in m['finds'])
        spirits += len(m['spirithieves'])
        total += len(m['chests'])
        icons += len(m['puzzles'])
    with open(OUT_JSON, 'w', encoding='utf-8') as fp:
        json.dump(maps, fp, ensure_ascii=False, separators=(',', ':'))
    print(f'{len(maps)} floors, {total} chests, {icons} puzzle icons, {spirits} spirithieves, finds {dict(found)}')


if __name__ == '__main__':
    main()
