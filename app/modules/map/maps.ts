import type { Position } from '@app/lib/game-data'
import table from './maps.json'

// Game floor -> its map, drawn like the game's minimap from the game's own
// textures (public/maps), and where world coordinates land on it.
// maps.json and the pictures are built by scripts/map-textures.py
// (docs: Модуль карты.md).

export const mapUrl = (file: string) => `maps/${file}`

export interface MapLayer {
  /** the game's map layer (MapLayerID) */
  layer: number
  /** roads of this layer, transparent, over the background */
  file: string
  /** "1F", "-1", "layout"… */
  name: string
}

/** A part of the map: its world rectangle and its place on the game's canvas. */
interface Area {
  cx: number
  cz: number
  sx: number
  sz: number
  /** lower-left corner on the canvas, px (py from the bottom) */
  px: number
  py: number
  /** canvas px per meter */
  s: number
  /** map layers drawn in this area (towers: one floor per area over the same ground) */
  layers: number[]
}

export type ChestKind = 'basic' | 'bountiful' | 'precious' | 'puzzle'
/** Collectibles: Amphoreus bugs, Penacony origami birds, nameless wispae (golden chenling stickers) */
export type FindKind = 'nymph' | 'origami' | 'wisp'
export type MarkKind = ChestKind | FindKind | 'spirithief'
/** A MazeChest: [id, x, z (m), map layer (0 unknown), kind]; puzzle rewards with no icon are 'puzzle' */
export type Chest = [number, number, number, number, ChestKind]
/**
 * A puzzle the game marks with its puzzle icon: ["group:id", x, z, layer,
 * PropStates without the icon (solved), reward chest id or 0, start]. The
 * point is where it ends (the reward appears); start [x, z, layer] is its
 * mechanism when that stands apart (the Golden Spear), else null.
 */
export type PuzzleIcon = [string, number, number, number, number[], number, [number, number, number] | null]

export interface FloorMap {
  /** Russian location name */
  name: string
  width: number
  height: number
  /** picture px per canvas px */
  scale: number
  /** degrees the world is turned on the map */
  rot: number
  canvasHeight: number
  background: string | null
  areas: Area[]
  layers: MapLayer[]
  /** objects with a known layer: [x, y, z, layer] in meters */
  marks: [number, number, number, number][]
  /** from scripts/map-markers.py */
  chests: Chest[]
  puzzles: PuzzleIcon[]
  /** collectibles: [kind, "group:id" of the thing, x, z, layer, its PropStates when collected] */
  finds: [FindKind, string, number, number, number, number[]][]
  /** Amphoreus spirithieves: ["group:id" of the reward chest prop, x, z, layer, its PropStates when taken, MazeChest id or 0] */
  spirithieves: [string, number, number, number, number[], number][]
}

const MAPS = table as unknown as Record<string, FloorMap>

export const mapForFloor = (floor: number | null | undefined): FloorMap | null =>
  floor == null ? null : (MAPS[String(floor)] ?? null)

/** Areas smallest first: a point belongs to the smallest that holds it. */
const sorted = new WeakMap<FloorMap, Area[]>()
function areasOf(map: FloorMap): Area[] {
  let list = sorted.get(map)
  if (!list) {
    list = map.areas.slice().sort((a, b) => a.sx * a.sz - b.sx * b.sz)
    sorted.set(map, list)
  }
  return list
}

/** Player position (1/1000 m) -> picture px, on the player's floor. */
export const toPicture = (map: FloorMap, p: Position) =>
  worldToPicture(map, p.x / 1000, p.z / 1000, playerGameLayer(map, p))

/**
 * World point (m) -> picture px. The point takes the smallest area of its
 * map layer that holds it, else the largest of that layer. Same as
 * scripts/map-textures.py.
 */
export function worldToPicture(map: FloorMap, x: number, z: number, layer: number | null): { x: number; y: number } {
  const r = (map.rot * Math.PI) / 180
  const local = (a: Area) => {
    const dx = x - a.cx
    const dz = z - a.cz
    return { rx: dx * Math.cos(r) - dz * Math.sin(r), rz: dx * Math.sin(r) + dz * Math.cos(r) }
  }
  const all = areasOf(map)
  const own = all.filter((a) => layer && a.layers.includes(layer))
  const areas = own.length ? own : all
  const area =
    areas.find((a) => {
      const { rx, rz } = local(a)
      return Math.abs(rx) <= a.sx / 2 && Math.abs(rz) <= a.sz / 2
    }) ?? areas[areas.length - 1]
  const { rx, rz } = local(area)
  const cx = area.px + (rx + area.sx / 2) * area.s
  const cy = map.canvasHeight - (area.py + (rz + area.sz / 2) * area.s)
  return { x: cx * map.scale, y: cy * map.scale }
}

/** Picture px per meter (for zoom in meters). */
export const pxPerMeter = (map: FloorMap) => Math.max(...map.areas.map((a) => a.s)) * map.scale

/** Heading on screen (clockwise degrees from up) for the game's yaw. */
export const headingOnScreen = (map: FloorMap, p: Position) => p.rotY / 1000 - map.rot

/** Order of floors from the lowest: "-2" < "-1" < "1F" < "2F"; others keep their place. */
function level(name: string): number {
  const m = /^(-?\d+)$|^(\d+)F$/.exec(name)
  if (!m) return 0
  return m[1] ? Number(m[1]) : Number(m[2])
}

export const floorsLowToHigh = (layers: MapLayer[]) => [...layers].sort((a, b) => level(a.name) - level(b.name))

export function floorLabel(name: string): string {
  if (name === 'layout') return 'Карта'
  const n = level(name)
  return n < 0 ? `Этаж −${-n}` : `Этаж ${n}`
}

/** Height counts for more than distance on the ground: floors are stacked. */
const HEIGHT_WEIGHT = 3
const VOTERS = 7

/**
 * The game's map layer the player is on. The game sends it with movement in
 * some places only (often 0); otherwise the nearest objects with a known
 * layer vote, height weighing three times.
 */
export function playerGameLayer(map: FloorMap, p: Position | null): number | null {
  if (!p) return null
  if (p.layer) return p.layer
  if (!map.marks.length) return null
  const x = p.x / 1000
  const y = p.y / 1000
  const z = p.z / 1000
  const near: [number, number][] = []
  for (const [mx, my, mz, l] of map.marks) {
    const d = (mx - x) ** 2 + (mz - z) ** 2 + ((my - y) * HEIGHT_WEIGHT) ** 2
    if (near.length < VOTERS) near.push([d, l])
    else if (d < near[VOTERS - 1][0]) near[VOTERS - 1] = [d, l]
    else continue
    near.sort((a, b) => a[0] - b[0])
  }
  const votes = new Map<number, number>()
  for (const [d, l] of near) votes.set(l, (votes.get(l) ?? 0) + 1 / (1 + Math.sqrt(d)))
  return [...votes].sort((a, b) => b[1] - a[1])[0]?.[0] ?? null
}

/** The floor the player is on; the ground floor if unknown. */
export function playerLayer(map: FloorMap, p: Position | null): MapLayer {
  const gl = playerGameLayer(map, p)
  const byGame = gl != null ? map.layers.find((l) => l.layer === gl) : undefined
  if (byGame) return byGame
  const floors = floorsLowToHigh(map.layers)
  return floors.find((f) => level(f.name) === 1) ?? floors.find((f) => level(f.name) >= 0) ?? floors[0]
}
