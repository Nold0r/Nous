import { useMemo, useState } from 'react'
import { worldToPicture, type Chest, type FloorMap, type MapLayer } from './maps'

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
  chest: Chest
  /** picture px */
  x: number
  y: number
}

/** Picture positions of the floor's chests, once per map. */
export function useChestMarks(map: FloorMap): Mark[] {
  return useMemo(
    () => map.chests.map((chest) => ({ chest, ...worldToPicture(map, chest[1], chest[2], chest[3] || null) })),
    [map],
  )
}

/** Marks to draw: on the floor shown (or of unknown floor), as the filter says. */
export function visibleMarks(marks: Mark[], map: FloorMap, layer: MapLayer, filter: MarkFilter, opened: Set<number> | null) {
  return marks.filter(({ chest: [id, , , l, kind] }) => {
    if (map.layers.length > 1 && l && l !== layer.layer) return false
    if (!(kind === 'puzzle' ? filter.puzzles : filter.chests)) return false
    return filter.opened || !opened?.has(id)
  })
}
