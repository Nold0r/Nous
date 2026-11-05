"""Builds the map pictures and modules/map/maps.json from the game's own minimap
textures, the way the game draws its minimap. Docs: docs/Архитектура/Модуль карты.md

    python scripts/map-textures.py <TurnBasedGameData clone> <exported textures>

<exported textures> is the folder AnimeStudio exported the game's Texture2D
assets into, grouped by container (it holds assets/asbres/spriteoutput/mappics/
minimap/…). How to export: see the doc.

A floor's map (MapInfo_P*_F*.json) is a canvas of BackgroundMapSpriteSize px:
the background sprite covers it, and each visible area puts the road sprite of
the layer (RoadMapSpriteDict) over its rectangle (PositionInCanvas, Size ×
Scale). World (x, z) -> canvas is exact, so the player lands where the game
shows them.
"""
import argparse, glob, json, math, os, re
import numpy as np
from PIL import Image

APP = os.path.join(os.path.dirname(__file__), '..')
OUT_JSON = os.path.join(APP, 'modules', 'map', 'maps.json')
OUT_PICS = os.path.join(APP, 'public', 'maps')
# picture px per meter: sharp enough to read, small enough to ship
PX_PER_M = 4
WEBP_QUALITY = 80


def textmap(tb, lang):
    out = {}
    for f in os.listdir(os.path.join(tb, 'TextMap')):
        if re.fullmatch(rf'TextMap(Main)?{lang}(_\d+)?\.json', f):
            out.update(json.load(open(os.path.join(tb, 'TextMap', f), encoding='utf-8')))
    return out


def floor_objects(tb, plane, floor):
    """Objects of the floor: rows (x, y, z, map layer), layer 0 = unknown."""
    layer = {}
    baked = f'{tb}/Config/LevelOutput_Baked/Floor/P{plane}_F{floor}_Baked.json'
    if os.path.exists(baked):
        for dim in json.load(open(baked, encoding='utf-8'))['DimensionList']:
            for g in dim['GroupList']:
                for lst in ('PropList', 'MonsterList', 'AnchorList', 'NPCList'):
                    for o in g.get(lst) or []:
                        lid = (o.get('NavMap') or {}).get('MapLayerID') or 0
                        if lid > 0:
                            layer[(g['ID'], lst, o['ID'])] = lid
    rows = []
    for f in glob.glob(f'{tb}/Config/LevelOutput/SharedRuntimeGroup/Groups_P{plane}_F{floor}/*.json'):
        gid = int(re.search(r'_G(\d+)(?:_D\d+)?\.json$', f)[1])
        g = json.load(open(f, encoding='utf-8'))
        for lst in ('PropList', 'MonsterList', 'AnchorList', 'NPCList'):
            for o in g.get(lst) or []:
                if 'PosX' in o:
                    rows.append((o['PosX'], o.get('PosY', 0.0), o.get('PosZ', 0.0), layer.get((gid, lst, o.get('ID')), 0)))
    return np.array(rows) if rows else np.zeros((0, 4))


def areas(mi):
    """Visible map areas: world rectangle and place on the canvas (lower-left
    corner px/py, py from the bottom; s canvas px per meter)."""
    bw, bh = mi['BackgroundMapSpriteSize']['X'], mi['BackgroundMapSpriteSize']['Y']
    out = []
    for a in mi.get('AreaList', []):
        if a.get('IsHidden'):
            continue
        v = a['MinimapVolume']
        pos = a.get('PositionInCanvas') or {}
        out.append(dict(cx=v['Center']['X'], cz=v['Center']['Z'], sx=v['Size']['X'], sz=v['Size']['Z'],
                        px=pos.get('X', 0) * bw, py=pos.get('Y', 0) * bh,
                        s=mi['Scale'] * (a.get('ScaleInCanvas') or 1),
                        roads={int(k): v for k, v in (v.get('RoadMapSpriteDict') or {}).items()},
                        layers=a.get('MapLayerList') or []))
    return out


def to_canvas(ars, bh, x, z, rot, layer=None):
    """World (x, z) arrays -> canvas px (x right, y down). A point takes the
    smallest area of its map layer that holds it, else the largest one of its
    layer; towers keep each floor's area over the same ground. Layer 0 or one
    no area lists: any area. Same as maps.ts."""
    vis = sorted(ars, key=lambda a: a['sx'] * a['sz'])
    layer = np.zeros(len(x), int) if layer is None else layer.astype(int)
    listed = {l for a in ars for l in a['layers']}
    free = ~np.isin(layer, list(listed))
    cx, cy = np.full(len(x), np.nan), np.full(len(x), np.nan)
    c, s = math.cos(math.radians(rot)), math.sin(math.radians(rot))
    for a in reversed(vis):  # largest first, smaller ones overwrite
        dx, dz = x - a['cx'], z - a['cz']
        rx, rz = dx * c - dz * s, dx * s + dz * c
        mine = free | np.isin(layer, a['layers'])
        inside = (np.abs(rx) <= a['sx'] / 2) & (np.abs(rz) <= a['sz'] / 2)
        take = mine & (inside | np.isnan(cx))
        cx[take] = a['px'] + (rx[take] + a['sx'] / 2) * a['s']
        cy[take] = bh - (a['py'] + (rz[take] + a['sz'] / 2) * a['s'])
    return cx, cy


def floor_label(sprite, layer, count):
    """'B1', '1F', '2F'… from the road sprite's name, else the layer's number."""
    name = os.path.splitext(os.path.basename(sprite))[0]
    m = re.search(r'(?:_|Road)(B\d|\d+F|F\d)(?:$|_)', name, re.I)
    if m:
        t = m[1].upper()
        return f'-{t[1:]}' if t.startswith('B') else t.replace('F', '') + 'F' if t.startswith('F') else t
    return f'{layer}F' if count > 1 else 'layout'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('tbgd')
    ap.add_argument('textures')
    ap.add_argument('--only', help='comma-separated floor ids, for checking')
    args = ap.parse_args()
    tb = args.tbgd
    only = set(args.only.split(',')) if args.only else None
    root = os.path.join(args.textures, 'assets', 'asbres', 'spriteoutput', 'mappics', 'minimap')
    files = {f.lower(): os.path.join(root, f) for f in os.listdir(root)}
    texture = lambda path: Image.open(files[os.path.basename(path).lower()]).convert('RGBA')
    has = lambda path: path and os.path.basename(path).lower() in files

    RU = textmap(tb, 'RU')
    planes = {p['PlaneID']: p for p in json.load(open(f'{tb}/ExcelOutput/MazePlane.json', encoding='utf-8'))}
    os.makedirs(OUT_PICS, exist_ok=True)
    if not only:
        for old in glob.glob(os.path.join(OUT_PICS, '*.webp')):
            os.remove(old)

    out = json.load(open(OUT_JSON, encoding='utf-8')) if only and os.path.exists(OUT_JSON) else {}
    total = 0
    for fn in sorted(os.listdir(f'{tb}/Config/LevelOutput/Map')):
        m = re.match(r'MapInfo_P(\d+)_F(\d+)\.json', fn)
        plane, floor = int(m[1]), int(m[2])
        if only and str(floor) not in only:
            continue
        mi = json.load(open(f'{tb}/Config/LevelOutput/Map/{fn}', encoding='utf-8'))
        if not mi.get('BackgroundMapSpriteSize'):
            continue
        ars = areas(mi)
        layer_ids = sorted({l for a in ars for l, road in a['roads'].items() if has(road)})
        if not ars or not layer_ids:
            continue
        name = RU.get(str(planes.get(plane, {}).get('PlaneName', {}).get('Hash')), '')
        if not name or name.startswith('{'):
            continue
        bw, bh = mi['BackgroundMapSpriteSize']['X'], mi['BackgroundMapSpriteSize']['Y']
        # picture px per canvas px; small maps (the Express) keep their own detail
        R = max(PX_PER_M / mi['Scale'], min(1.0, 1500 / max(bw, bh)))
        W, H = round(bw * R), round(bh * R)

        # background once per floor, roads per layer on transparent: the app
        # stacks them like the game does
        bg = None
        if has(mi.get('BackgroundMapSprite')):
            bg = texture(mi['BackgroundMapSprite']).resize((W, H), Image.LANCZOS)
        pics = {}
        for layer in layer_ids:
            img = Image.new('RGBA', (W, H), (0, 0, 0, 0))
            for a in ars:
                road = a['roads'].get(layer)
                if not has(road):
                    continue
                w, h = a['sx'] * a['s'], a['sz'] * a['s']
                top = bh - (a['py'] + h)
                img.alpha_composite(texture(road).resize((round(w * R), round(h * R)), Image.LANCZOS),
                                    (round(a['px'] * R), round(top * R)))
            pics[layer] = img

        # the game turns some maps: keep the turn under which the floor's
        # objects stand on roads
        pts = floor_objects(tb, plane, floor)
        if not len(pts):
            continue
        roads = np.logical_or.reduce([np.array(p)[:, :, 3] > 0 for p in pics.values()])
        rot0 = mi.get('Rotation') or 0
        best = None
        for rot in sorted({0, rot0, -rot0}):
            cx, cy = to_canvas(ars, bh, pts[:, 0], pts[:, 2], rot, pts[:, 3])
            px, py = (cx * R).astype(int), (cy * R).astype(int)
            ok = (px >= 0) & (px < W) & (py >= 0) & (py < H)
            hit = np.zeros(len(pts), bool)
            hit[ok] = roads[py[ok], px[ok]]
            if not best or hit.mean() > best[0]:
                best = (hit.mean(), rot)
        hit, rot = best

        background = None
        if bg:
            background = f'{floor}-bg.webp'
            bg.save(os.path.join(OUT_PICS, background), 'WEBP', quality=WEBP_QUALITY, method=6)
            total += os.path.getsize(os.path.join(OUT_PICS, background))
        layers = []
        for layer in layer_ids:
            file = f'{floor}-{layer}.webp'
            pics[layer].save(os.path.join(OUT_PICS, file), 'WEBP', quality=WEBP_QUALITY, method=6)
            total += os.path.getsize(os.path.join(OUT_PICS, file))
            road = next(a['roads'][layer] for a in ars if layer in a['roads'])
            layers.append(dict(layer=layer, file=file, name=floor_label(road, layer, len(layer_ids))))

        # objects with a known layer, one per 2 m cell: the player's floor is
        # voted by the nearest of them
        marks, seen = [], set()
        if len(layer_ids) > 1:
            for x, y, z, l in pts:
                cell = (round(x / 2), round(y / 2), round(z / 2), int(l))
                if l > 0 and cell not in seen:
                    seen.add(cell)
                    marks.append([round(x), round(y), round(z), int(l)])

        out[str(floor)] = dict(
            name=name, width=W, height=H, scale=R, background=background, rot=rot, canvasHeight=bh,
            areas=[dict(cx=round(a['cx'], 3), cz=round(a['cz'], 3), sx=round(a['sx'], 3), sz=round(a['sz'], 3),
                        px=round(a['px'], 2), py=round(a['py'], 2), s=round(a['s'], 5), layers=a['layers']) for a in ars],
            layers=layers, marks=marks,
        )
        print(f'{floor} {name[:40]:40s} {W}x{H} layers {[l["name"] for l in layers]} on roads {hit:.2f} rot {rot}', flush=True)

    with open(OUT_JSON, 'w', encoding='utf-8') as fp:
        json.dump(out, fp, ensure_ascii=False, separators=(',', ':'))
    print(f'{len(out)} floors, pictures {total // 1024 // 1024} MB, table {os.path.getsize(OUT_JSON) // 1024} KB')


if __name__ == '__main__':
    main()
