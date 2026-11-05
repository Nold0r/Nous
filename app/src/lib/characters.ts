import type { Avatar, AvatarPath, Inventory, LightCone } from './inventory'
import {
  RELIC_SLOTS,
  viewRelic,
  type CharacterTables,
  type GameTables,
  type Property,
  type RelicView,
} from './game-tables'

// A character as the screens show it: the stored build (lib/inventory.ts)
// joined with the game tables, plus totals computed like the game's
// character screen. Docs: docs/Архитектура/Модуль персонажей.md

/** Multi-path characters whose path ids aren't their base id (March 7th). */
const BASE_OF_PATH: Record<number, number> = { 1224: 1001 }

const isTrailblazer = (id: number) => id >= 8001 && id <= 8999

/** The shared level/ascension record of a path avatar id. */
export function baseAvatar(inventory: Inventory, pathId: number): Avatar | undefined {
  const avatars = Object.values(inventory.avatars)
  return (
    inventory.avatars[pathId] ??
    avatars.find((a) => a.currentPath === pathId) ??
    inventory.avatars[BASE_OF_PATH[pathId]] ??
    (isTrailblazer(pathId) ? avatars.find((a) => isTrailblazer(a.baseAvatarId)) : undefined)
  )
}

/**
 * Path avatar ids to show, one card each. The Trailblazer has a build per
 * path and gender; the game keeps both genders' entries, so only the
 * paths of the current gender (same parity as the current path) are kept.
 */
export function shownPaths(inventory: Inventory): number[] {
  const trailblazer = Object.values(inventory.avatars).find((a) => isTrailblazer(a.baseAvatarId))
  const parity = trailblazer && (trailblazer.currentPath || trailblazer.baseAvatarId) % 2
  return Object.keys(inventory.avatarPaths)
    .map(Number)
    .filter((id) => !isTrailblazer(id) || id % 2 === parity)
    .filter((id) => baseAvatar(inventory, id))
}

export interface SkillView {
  anchor: string
  name: string
  typeText: string
  icon: string
  level: number
  /** levels added by eidolons */
  bonus: number
  maxLevel: number
}

export interface SetView {
  setId: string
  name: string
  icon: string
  count: number
  /** active bonuses: 2 and/or 4 pieces */
  active: number[]
}

export interface StatLineView {
  /** key, also the order in STAT_ORDER */
  key: StatKey
  name: string
  /** StatIcon name */
  icon: string
  value: number
  /** part from the character and light cone base (HP, ATK, DEF, SPD) */
  base?: number
  percent: boolean
}

export type StatKey =
  | 'hp'
  | 'atk'
  | 'def'
  | 'spd'
  | 'critRate'
  | 'critDmg'
  | 'breakEffect'
  | 'heal'
  | 'energy'
  | 'effectHit'
  | 'effectRes'
  | 'dmgBonus'
  | 'elation'

export interface CharacterView {
  /** path avatar id */
  id: number
  name: string
  rarity: number
  element: string
  elementName: string
  path: string
  pathName: string
  avatar: Avatar
  build: AvatarPath
  /** the path the multi-path character currently uses */
  isCurrentPath: boolean
  lightCone: { cone: LightCone; name: string; rarity: number; icon: string; skill?: string } | null
  /** slot index 0 head … 5 rope */
  relics: (RelicView | null)[]
  sets: SetView[]
  skills: SkillView[]
  /** stat bonuses of the unlocked minor traces, summed by type */
  traceBonuses: Property[]
  stats: StatLineView[]
}

const sum = (props: Property[]) => {
  const out: Record<string, number> = {}
  for (const p of props) out[p.type] = (out[p.type] ?? 0) + p.value
  return out
}

const grow = (g: [number, number] | undefined, level: number) => (g ? g[0] + g[1] * (level - 1) : 0)

/** Joins a path avatar's build with the tables; null if the tables don't know it (newer game). */
export function viewCharacter(
  t: GameTables,
  ct: CharacterTables,
  inventory: Inventory,
  pathId: number,
  nickname: string | null,
): CharacterView | null {
  const build = inventory.avatarPaths[pathId]
  const avatar = baseAvatar(inventory, pathId)
  const def = t.characters[pathId]
  if (!build || !avatar || !def) return null

  // light cone
  const cone = build.lightCone != null ? inventory.lightCones[build.lightCone] : undefined
  const coneDef = cone && t.lightCones[cone.tid]
  const coneRank = cone && ct.lightConeRanks[cone.tid]
  const lightCone = cone
    ? { cone, name: coneDef?.name ?? `Конус ${cone.tid}`, rarity: coneDef?.rarity ?? 3, icon: coneDef?.icon ?? '', skill: coneRank?.skill }
    : null

  // relics by slot
  const relics: (RelicView | null)[] = RELIC_SLOTS.map((_, i) => {
    const uid = build.relics[i + 1]
    const relic = uid != null ? inventory.relics[uid] : undefined
    return relic ? viewRelic(t, relic) : null
  })
  const counts = new Map<string, number>()
  for (const r of relics) if (r) counts.set(r.setId, (counts.get(r.setId) ?? 0) + 1)
  const sets: SetView[] = [...counts]
    .map(([setId, count]) => ({
      setId,
      name: t.relicSets[setId]?.name ?? setId,
      icon: t.relicSets[setId]?.icon ?? '',
      count,
      active: [2, 4].filter((n) => count >= n),
    }))
    .sort((a, b) => b.count - a.count)

  // traces and skills (stored keys are point numbers: "1" = Point01)
  const tree = ct.traces[pathId] ?? {}
  const levelOf = (anchor: string) => build.skillTree[String(Number(anchor.slice(5)))] ?? 0
  const traceBonuses = Object.entries(tree)
    .filter(([anchor, p]) => p.properties.length > 0 && levelOf(anchor) > 0)
    .flatMap(([, p]) => p.properties)
  const eidolons = (ct.eidolons[pathId] ?? []).slice(0, build.eidolon)
  const skills: SkillView[] = Object.entries(tree)
    .filter(([anchor, p]) => p.skill && p.maxLevel > 1 && levelOf(anchor) > 0)
    .map(([anchor, p]) => {
      const skill = ct.skills[p.skill!]
      const bonus = eidolons.reduce((n, e) => n + (e.skills.find((s) => s.id === p.skill)?.num ?? 0), 0)
      return {
        anchor,
        name: skill?.name ?? anchor,
        typeText: skill?.typeText ?? '',
        icon: skill?.icon ?? p.icon,
        level: levelOf(anchor),
        bonus,
        maxLevel: p.maxLevel,
      }
    })

  // totals
  const bonuses = sum([
    ...relics.flatMap((r) => (r ? [r.main, ...r.subs].flatMap((s) => (s ? [{ type: s.property, value: s.value }] : [])) : [])),
    ...traceBonuses,
    ...sets.flatMap((s) => s.active.flatMap((n) => t.relicSets[s.setId]?.properties[n / 2 - 1] ?? [])),
    ...(cone && coneRank ? (coneRank.properties[cone.superimposition - 1] ?? []) : []),
  ])
  const promo = ct.promotions[pathId]?.[avatar.ascension]
  const lcPromo = cone ? ct.lightConePromotions[cone.tid]?.[cone.ascension] : undefined
  const base = (k: 'hp' | 'atk' | 'def') => grow(promo?.[k], avatar.level) + (cone ? grow(lcPromo?.[k], cone.level) : 0)
  const b = (type: string) => bonuses[type] ?? 0
  const scaled = (k: 'hp' | 'atk' | 'def', ratio: string, delta: string) => base(k) * (1 + b(ratio)) + b(delta)
  const spdBase = grow(promo?.spd, avatar.level)

  const prop = (type: string) => t.properties[type]
  const line = (key: StatKey, type: string, value: number, percent: boolean, baseValue?: number): StatLineView => ({
    key,
    name: prop(type)?.name ?? type,
    icon: prop(type)?.icon ?? '',
    value,
    base: baseValue,
    percent,
  })
  const dmgType = `${def.element}AddedRatio`
  const stats = [
    line('hp', 'MaxHP', scaled('hp', 'HPAddedRatio', 'HPDelta'), false, base('hp')),
    line('atk', 'Attack', scaled('atk', 'AttackAddedRatio', 'AttackDelta'), false, base('atk')),
    line('def', 'Defence', scaled('def', 'DefenceAddedRatio', 'DefenceDelta'), false, base('def')),
    line('spd', 'Speed', spdBase * (1 + b('SpeedAddedRatio')) + b('SpeedDelta'), false, spdBase),
    line('critRate', 'CriticalChanceBase', grow(promo?.critRate, 1) + b('CriticalChanceBase') + b('CriticalChance'), true),
    line('critDmg', 'CriticalDamageBase', grow(promo?.critDmg, 1) + b('CriticalDamageBase') + b('CriticalDamage'), true),
    line('breakEffect', 'BreakDamageAddedRatioBase', b('BreakDamageAddedRatioBase'), true),
    line('heal', 'HealRatioBase', b('HealRatioBase'), true),
    line('energy', 'SPRatioBase', 1 + b('SPRatioBase'), true),
    line('effectHit', 'StatusProbabilityBase', b('StatusProbabilityBase'), true),
    line('effectRes', 'StatusResistanceBase', b('StatusResistanceBase'), true),
    line('dmgBonus', dmgType, b(dmgType) + b('AllDamageTypeAddedRatio'), true),
    // Elation path stat (4.x): shown only when something gives it
    ...(b('ElationDamageAddedRatioBase') ? [line('elation', 'ElationDamageAddedRatioBase', b('ElationDamageAddedRatioBase'), true)] : []),
  ]

  const pathName = ct.paths[def.path]?.name ?? def.path
  return {
    id: pathId,
    name: characterDisplayName(def.name, nickname),
    rarity: def.rarity,
    element: def.element,
    elementName: ct.elements[def.element]?.name ?? def.element,
    path: def.path,
    pathName,
    avatar,
    build,
    isCurrentPath: avatar.currentPath === 0 || avatar.currentPath === pathId,
    lightCone,
    relics,
    sets,
    skills,
    traceBonuses: Object.entries(sum(traceBonuses)).map(([type, value]) => ({ type, value })),
    stats,
  }
}

/** "{NICKNAME}" in the Trailblazer's name is the player's nickname. */
export function characterDisplayName(name: string, nickname: string | null): string {
  return name.includes('{NICKNAME}') ? name.replace('{NICKNAME}', nickname || 'Первопроходец') : name
}

/**
 * Stat value as the game's character screen shows it: whole numbers rounded
 * down (SPD too), the rest in % with one decimal. `precise` keeps one
 * decimal for SPD (tooltips: 107.6 is not 107 when planning breakpoints).
 */
export function formatTotal(s: { key?: StatKey; value: number; percent: boolean }, precise = false): string {
  if (s.percent) return `${(Math.floor(s.value * 1000) / 10).toLocaleString('ru-RU')}%`
  if (precise && s.key === 'spd') return (Math.floor(s.value * 10) / 10).toLocaleString('ru-RU')
  return Math.floor(s.value).toLocaleString('ru-RU')
}

/** Highest ascension cap for a level: 20, 30 … 80. */
export const maxLevelOf = (ascension: number) => 20 + ascension * 10
