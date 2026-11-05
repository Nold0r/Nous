import { useEffect, useState } from 'react'
import type { Affix, Relic } from './inventory'

// Game tables (Russian names, stat values) made by `npm run game-data` from
// StarRailRes. Loaded on first use: the file is ~600 KB and only the
// inventory screens need it. Pictures load from StarRailRes by their path.

export interface GameTables {
  relics: Record<string, { set: string; slot: RelicSlot; rarity: number; mainGroup: string; subGroup: string; icon: string }>
  relicSets: Record<string, { name: string; desc: string[]; icon: string }>
  /** group -> affix id -> property */
  mainAffixes: Record<string, Record<string, AffixDef>>
  subAffixes: Record<string, Record<string, AffixDef>>
  /** icon: file name in assets/icons/property */
  properties: Record<string, { name: string; percent: boolean; icon: string }>
  lightCones: Record<string, { name: string; rarity: number; path: string; icon: string }>
  items: Record<string, { name: string; rarity: number; type: string; subType: string; icon: string }>
  characters: Record<string, { name: string; rarity: number; path: string; element: string }>
}

interface AffixDef {
  property: string
  base: number
  step: number
}

export type RelicSlot = 'HEAD' | 'HAND' | 'BODY' | 'FOOT' | 'NECK' | 'OBJECT'

/** In the game's order. */
export const RELIC_SLOTS: { id: RelicSlot; name: string }[] = [
  { id: 'HEAD', name: 'Голова' },
  { id: 'HAND', name: 'Руки' },
  { id: 'BODY', name: 'Тело' },
  { id: 'FOOT', name: 'Ноги' },
  { id: 'NECK', name: 'Сфера' },
  { id: 'OBJECT', name: 'Канат' },
]

const STAR_RAIL_RES = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/'

/** Picture from StarRailRes by its table path, e.g. "icon/relic/101_0.png". */
export const resUrl = (path: string) => STAR_RAIL_RES + path

let loading: Promise<GameTables> | null = null
let loaded: GameTables | null = null

function loadTables(): Promise<GameTables> {
  loading ??= import('../assets/data/game-data.json').then((m) => (loaded = m.default as unknown as GameTables))
  return loading
}

/** The tables, or null while they load. */
export function useGameTables(): GameTables | null {
  const [tables, setTables] = useState(loaded)
  useEffect(() => {
    if (!tables) loadTables().then(setTables).catch(console.error)
  }, [tables])
  return tables
}

export interface Stat {
  property: string
  name: string
  /** StatIcon name */
  icon: string
  value: number
  percent: boolean
  /** substat rolls (1 = the starting roll), absent for the main stat */
  rolls?: number
}

/** Stat value as the game shows it: "6,9%", "135", "2,6" for speed. */
export function formatStat(stat: Stat): string {
  if (stat.percent) return `${(Math.floor(stat.value * 1000) / 10).toLocaleString('ru-RU')}%`
  if (stat.property.startsWith('Speed')) return (Math.floor(stat.value * 10) / 10).toLocaleString('ru-RU')
  return String(Math.floor(stat.value))
}

function stat(t: GameTables, def: AffixDef | undefined, value: (d: AffixDef) => number): Stat | null {
  if (!def) return null
  const p = t.properties[def.property]
  return { property: def.property, name: p?.name ?? def.property, icon: p?.icon ?? '', value: value(def), percent: p?.percent ?? false }
}

/** A relic with names and values resolved; null when the tables don't know it (newer game). */
export interface RelicView {
  relic: Relic
  setId: string
  setName: string
  slot: RelicSlot
  rarity: number
  icon: string
  main: Stat | null
  subs: Stat[]
  /** pending Variable Dice result */
  reforge: Stat[]
}

export function viewRelic(t: GameTables, relic: Relic): RelicView | null {
  const def = t.relics[relic.tid]
  if (!def) return null
  const subGroup = t.subAffixes[def.subGroup] ?? {}
  const subs = (list: Affix[] = []) =>
    list.flatMap((a) => {
      const s = stat(t, subGroup[a.affixId], (d) => a.cnt * d.base + a.step * d.step)
      return s ? [{ ...s, rolls: a.cnt }] : []
    })
  return {
    relic,
    setId: def.set,
    setName: t.relicSets[def.set]?.name ?? `Набор ${def.set}`,
    slot: def.slot,
    rarity: def.rarity,
    icon: def.icon,
    main: stat(t, t.mainAffixes[def.mainGroup]?.[relic.mainAffixId], (d) => d.base + d.step * relic.level),
    subs: subs(relic.subAffixes),
    reforge: subs(relic.reforgeSubAffixes),
  }
}

/** Round character picture (path avatar id) from StarRailRes. */
export const characterIcon = (avatarId: number) => resUrl(`icon/avatar/${avatarId}.png`)

/** Character name; Trailblazer paths carry the player's nickname. */
export function characterName(t: GameTables, avatarId: number, nickname: string | null): string {
  const name = t.characters[avatarId]?.name
  if (!name) return `Персонаж ${avatarId}`
  return name.includes('{NICKNAME}') ? name.replace('{NICKNAME}', nickname || 'Первопроходец') : name
}
