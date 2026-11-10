"""Checks the map marks against what the game showed: the map scan Nous
writes with NOUS_MAP_SCAN=1 (crates/nous-hsr/src/mapscan.rs) after a run
through the locations.

For each scanned floor:
  - chests the game placed that we don't mark, and ours it never placed
    where it loaded the rest of their group;
  - objects with a mark's map icon (puzzle, nymph, anchor...) that have no
    mark of that kind near them;
  - our marks on objects the game never placed in a loaded group (marked
    where nothing is, like readables taken for books);
  - the in-game map's chest counts against ours;
  - map icons on objects we don't mark at all, to find new kinds.

    python scripts/map-scan-check.py <TurnBasedGameData clone> <map-scan dir> [floor ...]

The report is plain text on stdout. Docs: docs/Архитектура/Модуль карты.md.
"""
import argparse
import collections
import glob
import importlib.util
import json
import math
import os
import re
import sys

HERE = os.path.dirname(__file__)
spec = importlib.util.spec_from_file_location('markers', os.path.join(HERE, 'map-markers.py'))
mk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mk)

# map icon -> the kind of our mark that should be near it
ICON_KIND = {**{i: 'puzzle' for i in mk.PUZZLE_ICONS}, **{i: k for i, (k, _) in mk.FINDS.items()},
             **{i: 'anchor' for i in mk.ANCHOR_ICONS}}
# a mark this close to the game's object is the object's mark
NEAR_M = 6


def datamine(tb, plane, floor, props):
    """(group, inst) -> (PropID, ChestID, icon ids, group name) of the floor's props."""
    out = {}
    for f in glob.glob(f'{tb}/Config/LevelOutput/SharedRuntimeGroup/Groups_P{plane}_F{floor}/*.json'):
        gid = int(re.search(r'_G(\d+)(?:_D\d+)?\.json$', f)[1])
        group = json.load(open(f, encoding='utf-8'))
        for o in group.get('PropList') or []:
            prop = props.get(o.get('PropID'), {})
            rules = o.get('OverrideMapStateIcons') or prop.get('MiniMapStateIcons') or []
            out[(gid, o['ID'])] = (o.get('PropID'), o.get('ChestID') or 0,
                                   {r['IconID'] for r in rules if r.get('IconID')}, group.get('GroupName', ''))
    return out


def our_marks(m):
    """[(kind, key or '', x, z)] in metres, chests by id apart."""
    marks = []
    chests = {}
    for cid, x, z, _, kind in m.get('chests', []):
        chests[cid] = (kind, x, z)
        marks.append((kind, '', x, z))
    for key, x, z, _, _, cid, start in m.get('puzzles', []):
        marks.append(('puzzle', key, x, z))
        if start:
            marks.append(('puzzle', '', start[0], start[1]))
        if cid:
            chests[cid] = ('puzzle', x, z)
    for f in m.get('finds', []):
        marks.append((f[0], f[1], f[2], f[3]))
    for key, x, z, _, _, cid in m.get('spirithieves', []):
        marks.append(('spirithief', key, x, z))
        if cid:
            chests[cid] = ('spirithief', x, z)
    return marks, chests


def check(floor, scan, m, dm, props):
    lines = []
    seen = scan.get('objects', {})
    loaded = {int(k.split(':')[0]) for k in seen}
    at = {k: (o['x'] / 1000, o['z'] / 1000) for k, o in seen.items()}
    marks, chests = our_marks(m or {})

    def near(kinds, p):
        return any(k in kinds and math.dist((x, z), p) <= NEAR_M for k, _, x, z in marks)

    # chests
    placed = {}
    for k, o in seen.items():
        g, i = map(int, k.split(':'))
        cid = dm.get((g, i), (0, 0))[1]
        if o['kind'] == 'prop' and cid:
            placed[cid] = k
    missing = sorted(set(placed) - set(chests))
    if missing:
        lines.append(f'  сундуки в игре без метки: {len(missing)}: ' + ', '.join(
            f'{c} ({placed[c]} @ {at[placed[c]][0]:.0f},{at[placed[c]][1]:.0f})' for c in missing))
    chest_group = {cid: g for (g, _), (_, cid, _, _) in dm.items() if cid}
    never = sorted(c for c in chests if c not in placed and chest_group.get(c) in loaded)
    if never:
        lines.append(f'  наши сундуки, которых игра не поставила (группа загружена): {len(never)}: '
                     + ', '.join(f'{c} [{chests[c][0]}]' for c in never))

    # objects with a mark's icon and no mark near
    for k, o in sorted(seen.items()):
        g, i = map(int, k.split(':'))
        icons = dm.get((g, i), (0, 0, set()))[2] or {r.get('IconID') for r in props.get(o['id'], {}).get('MiniMapStateIcons') or []} - {None}
        kinds = {ICON_KIND[c] for c in icons if c in ICON_KIND}
        name = (props.get(o['id'], {}).get('JsonPath') or '').split('/')[-1]
        if 'PlaceHolder' in name and o.get('state') in mk.PLACEHOLDER_HIDDEN:
            continue  # the game shows no icon there now
        prop = props.get(o['id'], {})
        if o.get('state') in (mk.icon_states(prop.get('MiniMapStateIcons') or [], prop.get('PropStateList') or []) or []):
            continue  # nor in this state (a broken spear)
        if kinds and not near(kinds, at[k]):
            lines.append(f'  {"/".join(sorted(kinds))} в игре без метки: {k} {name} @ {at[k][0]:.0f},{at[k][1]:.0f} state {o.get("state")}')

    # our marks on objects that never came
    for kind, key, x, z in marks:
        if not key:
            continue
        g = int(key.split(':')[0])
        # a find names each variant of its thing: one is enough
        if g in loaded and not any(k in seen for k in key.split()):
            lines.append(f'  {kind} без объекта в игре: {key} @ {x:.0f},{z:.0f} ({dm.get(tuple(map(int, key.split()[0].split(":"))), (0, 0, 0, "?"))[3]})')

    # the in-game map's chest counts
    reply = scan.get('map')
    if reply and reply.get('chests'):
        ours = collections.Counter('puzzle' if v[0] == 'puzzle' else 'chest' for v in chests.values())
        game = ' '.join(f'тип {t}: {a}/{b}' for t, a, b in reply['chests'])
        lines.append(f'  карта игры: {game}; у нас {len(chests)} (из них за головоломки {ours["puzzle"]})')

    # icons we draw nothing for
    other = collections.Counter()
    for k, o in seen.items():
        g, i = map(int, k.split(':'))
        icons = dm.get((g, i), (0, 0, set()))[2]
        for c in icons:
            if c not in ICON_KIND:
                other[c] += 1
    if other:
        lines.append('  другие значки карты (id: сколько): ' + ', '.join(f'{c}: {n}' for c, n in sorted(other.items())))
    return lines


def main():
    sys.stdout.reconfigure(encoding='utf-8')
    ap = argparse.ArgumentParser()
    ap.add_argument('tbgd')
    ap.add_argument('scan')
    ap.add_argument('floors', nargs='*')
    a = ap.parse_args()
    tb = a.tbgd
    props = {p['ID']: p for p in json.load(open(f'{tb}/ExcelOutput/MazeProp.json', encoding='utf-8'))}
    maps = json.load(open(mk.OUT_JSON, encoding='utf-8'))
    files = sorted(glob.glob(os.path.join(a.scan, '*.json')))
    for f in files:
        scan = json.load(open(f, encoding='utf-8'))
        floor = str(scan['floor'])
        if a.floors and floor not in a.floors:
            continue
        m = maps.get(floor)
        plane = scan.get('plane') or int(floor[:-3])
        dm = datamine(tb, plane, floor, props)
        name = (m or {}).get('name', '(нет карты)')
        n = len(scan.get('objects', {}))
        print(f'{floor} {name}: объектов в скане {n}, в карте игры {"есть" if scan.get("map") else "нет"}')
        for line in check(floor, scan, m, dm, props):
            print(line)


if __name__ == '__main__':
    main()
