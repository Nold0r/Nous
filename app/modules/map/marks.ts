import { useMemo, useState } from 'react'
import { worldToPicture, type FloorMap, type MarkKind, type MapLayer } from './maps'

// Which chests and puzzles the map shows, and where. Drawn by MapMarks.tsx.

const SHOW_KEY = 'nous.map.marks'

export interface MarkFilter {
  chests: boolean
  puzzles: boolean
  nymphs: boolean
  origami: boolean
  wisps: boolean
  rabbits: boolean
  anchors: boolean
  spirits: boolean
  opened: boolean
}

const DEFAULT_FILTER: MarkFilter = {
  chests: true,
  puzzles: true,
  nymphs: true,
  origami: true,
  wisps: true,
  rabbits: true,
  anchors: true,
  spirits: true,
  opened: true,
}

function savedFilter(): MarkFilter {
  try {
    return { ...DEFAULT_FILTER, ...JSON.parse(localStorage.getItem(SHOW_KEY) ?? '{}') }
  } catch {
    return DEFAULT_FILTER
  }
}

/** The filter, remembered between visits. */
export function useMarkFilter(): [MarkFilter, (f: MarkFilter) => void] {
  const [filter, setFilter] = useState(savedFilter)
  const save = (f: MarkFilter) => {
    setFilter(f)
    try {
      localStorage.setItem(SHOW_KEY, JSON.stringify(f))
    } catch {
      // the choice just won't be remembered
    }
  }
  return [filter, save]
}

export interface Mark {
  key: string
  kind: MarkKind
  /** map layer, 0 unknown */
  layer: number
  /** MazeChest id: the chest, or the puzzle's reward; 0 none */
  chest: number
  /** "group:id" of the puzzle's icon prop */
  prop: string | null
  /** PropStates of `prop` that mean done: a puzzle without its icon, a caught nymph */
  hide: number[]
  /** where a puzzle with its own start ends: its reward */
  reward: boolean
  /** the other point of such a puzzle */
  pair: string | null
  /** picture px */
  x: number
  y: number
}

/** What the game told about the floor: opened chests and prop states. */
export interface Progress {
  opened: Set<number> | null
  props: Record<string, number> | null
}

/** Picture positions of the floor's chests and puzzles, once per map. */
export function useMarks(map: FloorMap): Mark[] {
  return useMemo(() => {
    const at = (x: number, z: number, layer: number) => worldToPicture(map, x, z, layer || null)
    return [
      ...map.chests.map(([id, x, z, layer, kind]) => ({
        key: `c${id}`, kind, layer, chest: id, prop: null, hide: [], reward: false, pair: null, ...at(x, z, layer),
      })),
      ...map.puzzles.flatMap(([prop, x, z, layer, hide, chest, start]): Mark[] => {
        const puzzle = { kind: 'puzzle' as const, chest, prop, hide }
        if (!start) return [{ ...puzzle, key: `p${prop}`, layer, reward: false, pair: null, ...at(x, z, layer) }]
        const [sx, sz, sl] = start
        return [
          { ...puzzle, key: `p${prop}`, layer: sl, reward: false, pair: `r${prop}`, ...at(sx, sz, sl) },
          { ...puzzle, key: `r${prop}`, layer, reward: true, pair: `p${prop}`, ...at(x, z, layer) },
        ]
      }),
      ...map.finds.map(([kind, prop, x, z, layer, found]) => ({
        key: `f${prop}${x}`, kind, layer, chest: 0, prop: prop || null, hide: found, reward: false, pair: null, ...at(x, z, layer),
      })),
      ...map.spirithieves.map(([prop, x, z, layer, taken, chest]) => ({
        key: `s${prop}${x}`, kind: 'spirithief' as const, layer, chest, prop: prop || null, hide: taken, reward: false, pair: null, ...at(x, z, layer),
      })),
    ]
  }, [map])
}

/**
 * Opened chest or solved puzzle; null when the game hasn't said. A reward
 * chest decides; else the icon's prop state, as the game hides the icon.
 */
export function isDone(m: Mark, { opened, props }: Progress): boolean | null {
  if (m.chest && opened) return opened.has(m.chest)
  const state = m.prop ? props?.[m.prop] : undefined
  return state == null ? null : m.hide.includes(state)
}

/**
 * A puzzle with no reward chest is there only once the game has reported its
 * prop: many sit in mission groups the game never loads for this player.
 */
export const isPresent = (m: Mark, { props }: Progress) =>
  m.kind !== 'puzzle' || m.chest !== 0 || !m.prop || props?.[m.prop] != null

const SHOWN_BY: Record<MarkKind, keyof MarkFilter> = {
  basic: 'chests',
  bountiful: 'chests',
  precious: 'chests',
  puzzle: 'puzzles',
  nymph: 'nymphs',
  origami: 'origami',
  wisp: 'wisps',
  rabbit: 'rabbits',
  anchor: 'anchors',
  spirithief: 'spirits',
}

/** Which toggle a mark belongs to. */
export const groupOf = (m: Mark) => SHOWN_BY[m.kind]

/** Marks to draw: on the floor shown (or of unknown floor), as the filter says. */
export function visibleMarks(marks: Mark[], map: FloorMap, layer: MapLayer, filter: MarkFilter, progress: Progress) {
  return marks.filter((m) => {
    if (!isPresent(m, progress)) return false
    if (map.layers.length > 1 && m.layer && m.layer !== layer.layer) return false
    if (!filter[groupOf(m)]) return false
    return filter.opened || !isDone(m, progress)
  })
}
