import { useMemo, useState } from 'react'
import { worldToPicture, type ChestKind, type FloorMap, type MapLayer } from './maps'

// Which chests and puzzles the map shows, and where. Drawn by MapMarks.tsx.

const SHOW_KEY = 'nous.map.marks'

export interface MarkFilter {
  chests: boolean
  puzzles: boolean
  opened: boolean
}

const DEFAULT_FILTER: MarkFilter = { chests: true, puzzles: true, opened: true }

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
  kind: ChestKind
  /** map layer, 0 unknown */
  layer: number
  /** MazeChest id: the chest, or the puzzle's reward; 0 none */
  chest: number
  /** "group:id" of the puzzle's icon prop */
  prop: string | null
  /** its PropStates without the icon: solved */
  hide: number[]
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
        key: `c${id}`, kind, layer, chest: id, prop: null, hide: [], ...at(x, z, layer),
      })),
      ...map.puzzles.map(([prop, x, z, layer, hide, chest]) => ({
        key: `p${prop}`, kind: 'puzzle' as const, layer, chest, prop, hide, ...at(x, z, layer),
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
export const isPresent = (m: Mark, { props }: Progress) => m.chest !== 0 || !m.prop || props?.[m.prop] != null

/** Marks to draw: on the floor shown (or of unknown floor), as the filter says. */
export function visibleMarks(marks: Mark[], map: FloorMap, layer: MapLayer, filter: MarkFilter, progress: Progress) {
  return marks.filter((m) => {
    if (!isPresent(m, progress)) return false
    if (map.layers.length > 1 && m.layer && m.layer !== layer.layer) return false
    if (!(m.kind === 'puzzle' ? filter.puzzles : filter.chests)) return false
    return filter.opened || !isDone(m, progress)
  })
}
