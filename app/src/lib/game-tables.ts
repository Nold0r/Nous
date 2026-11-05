import { useEffect, useState } from 'react'
import type { Affix, Relic } from './inventory'

// Game tables (Russian names, stat values) made by `npm run game-data` from
// StarRailRes. Loaded on first use: the file is ~600 KB and only the
// inventory screens need it. Pictures load from StarRailRes by their path.

export interface GameTables {
  relics: Record<string, { set: string; slot: RelicSlot; rarity: number; mainGroup: string; subGroup: string; icon: string }>
  /** properties: unconditional stats of the 2- and 4-piece bonuses */
  relicSets: Record<string, { name: string; desc: string[]; icon: string; properties: Property[][] }>
  /** group -> affix id -> property */
  mainAffixes: Record<string, Record<string, AffixDef>>
  subAffixes: Record<string, Record<string, AffixDef>>
  /** icon: file name in assets/icons/property */
  properties: Record<string, { name: string; percent: boolean; icon: string }>
  lightCones: Record<string, { name: string; rarity: number; path: string; icon: string }>
  items: Record<string, { name: string; rarity: number; type: string; subType: string; icon: string }>
  characters: Record<string, { name: string; rarity: number; path: string; element: string }>
}

/** A stat bonus as the game tables write it, e.g. { type: 'AttackAddedRatio', value: 0.04 }. */
export interface Property {
  type: string
  value: number
}

/** [base, step per level above 1] */
type Growth = [number, number]

/** Tables only the characters screen needs (game-data-characters.json). */
export interface CharacterTables {
  /** character id -> by ascension */
  promotions: Record<
    string,
    { hp: Growth; atk: Growth; def: Growth; spd: Growth; critRate: Growth; critDmg: Growth }[]
  >
  /** light cone id -> by ascension */
  lightConePromotions: Record<string, { hp: Growth; atk: Growth; def: Growth }[]>
  /** light cone id -> passive; properties by superimposition 1..5 */
  lightConeRanks: Record<string, { skill: string; desc: string; params: number[][]; properties: Property[][] }>
  /** character id -> "Point01" -> trace point */
  traces: Record<
    string,
    Record<string, { maxLevel: number; name: string; icon: string; skill?: string; properties: Property[] }>
  >
  skills: Record<string, { name: string; type: string; typeText: string; maxLevel: number; icon: string }>
  /** character id -> eidolons 1..6; skills: levels they add */
  eidolons: Record<string, { name: string; desc: string; icon: string; skills: { id: string; num: number }[] }[]>
  elements: Record<string, { name: string; color: string; icon: string }>
  paths: Record<string, { name: string; icon: string }>
  characterIcons: Record<string, { icon: string; portrait: string }>
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

/** Main stats a relic in this slot can roll, from the game tables (properties, e.g. "HPDelta"). */
export function slotMainStats(t: GameTables, slot: RelicSlot): Set<string> {
  const groups = new Set(Object.values(t.relics).filter((r) => r.slot === slot).map((r) => r.mainGroup))
  return new Set([...groups].flatMap((g) => Object.values(t.mainAffixes[g] ?? {}).map((a) => a.property)))
}

/**
 * Substats a relic in this slot can roll: every substat except a main stat
 * the slot always has (head: flat HP, hands: flat ATK) — a substat never
 * repeats the main stat.
 */
export function slotSubStats(t: GameTables, slot: RelicSlot): Set<string> {
  const all = new Set(Object.values(t.subAffixes).flatMap((g) => Object.values(g).map((a) => a.property)))
  const main = slotMainStats(t, slot)
  if (main.size === 1) all.delete([...main][0])
  return all
}

/** "Голова", "Сфера"… */
export const slotName = (slot: RelicSlot) => RELIC_SLOTS.find((s) => s.id === slot)?.name ?? slot

const STAR_RAIL_RES = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/'

/** Picture from StarRailRes by its table path, e.g. "icon/relic/101_0.png". */
export const resUrl = (path: string) => STAR_RAIL_RES + path

let loading: Promise<GameTables> | null = null
let loaded: GameTables | null = null

function loadTables(): Promise<GameTables> {
  loading ??= import('../assets/data/game-data.json').then((m) => (loaded = m.default as unknown as GameTables))
  return loading
}

let charactersLoading: Promise<CharacterTables> | null = null
let charactersLoaded: CharacterTables | null = null

function loadCharacterTables(): Promise<CharacterTables> {
  charactersLoading ??= import('../assets/data/game-data-characters.json').then(
    (m) => (charactersLoaded = m.default as unknown as CharacterTables),
  )
  return charactersLoading
}

/** Base stats, traces, skills and eidolons, or null while they load. */
export function useCharacterTables(): CharacterTables | null {
  const [tables, setTables] = useState(charactersLoaded)
  useEffect(() => {
    if (!tables) loadCharacterTables().then(setTables).catch(console.error)
  }, [tables])
  return tables
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
