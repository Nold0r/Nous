import { useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useLive, type Versioned } from './live'

// Game data beyond the inventory (see lib/inventory.ts): world, battles,
// warps, raw snapshots. Mirrors world.rs, battles.rs, gacha.rs, snapshots.rs.
// Docs: docs/Архитектура/API данных игры.md

// ---------- world ----------

/** 1/1000 m; heading in 1/1000 degree */
export interface Position {
  x: number
  y: number
  z: number
  rotY: number
  layer: number | null
}

export interface World {
  planeId: number | null
  floorId: number | null
  entryId: number | null
  player: Position | null
  /** floor id -> opened MazeChest ids */
  openedChests: Record<string, number[]>
  /** floor id -> chest id -> where the player stood when opening it */
  openedAt: Record<string, Record<string, Position>>
  /** floor id -> "group:config" -> PropState (8 anchor on, 12/13 chest closed/used, …) */
  props: Record<string, Record<string, number>>
}

const readWorld = () => invoke<Versioned<World>>('get_world')

/** Location and world progress; the player position updates live. */
export function useWorld(): World | null {
  return useLive(readWorld, {
    'world-changed': (_: World, next: World) => next,
    'player-moved': (w: World, player: Position | null) => ({ ...w, player }),
  })
}

// ---------- battles ----------

export interface BattleAvatar {
  avatarId: number
  level: number
  eidolon: number
  /** support character's owner uid, 0 for own characters */
  assistUid: number
  turns: number
  damage: number
  breakDamage: number
  heal: number
  shield: number
  damageTaken: number
  /** attack type (Normal, BPSkill, Ultra, Insert, DOT, …) -> damage */
  damageByType: Record<string, number>
  /** attack type -> biggest single hit */
  maxHitByType: Record<string, number>
  skillUses: Record<string, number>
  breaks: number
  deaths: number
  spUsed: number
  spGained: number
  /** action value this character accumulated */
  actionValue: number
  /** damage per action value of the fight (Veritas' DPAV), 0 if unknown */
  dpav: number
  relics: number[]
  lightCone: number | null
}

export interface BattleMonster {
  monsterId: number
  wave: number
  damage: number
  damageTaken: number
  turns: number
}

export interface Battle {
  battleId: number
  stageId: number
  result: 'win' | 'lose' | 'quit' | 'none'
  /** unix seconds */
  at: number
  costTime: number
  turns: number
  autoTurns: number
  ultimates: number
  rounds: number
  challengeScore: number
  /** action value the fight took */
  actionValue: number
  avatars: BattleAvatar[]
  monsters: BattleMonster[]
  /** client action log, in order; entity ids are battle-local (not verified yet) */
  actions: BattleAction[]
}

export interface BattleAction {
  turn: number
  actor: number
  target: number
  opType: number
  skillIndex: number
  counter: number
}

const readBattles = () => invoke<Versioned<Battle[]>>('get_battles')

/** Battle reports, newest last; a new one arrives as soon as a fight ends. */
export function useBattles(): Battle[] {
  return (
    useLive(readBattles, {
      'battle-finished': (list: Battle[], battle: Battle) => [...list, battle],
    }) ?? []
  )
}

// ---------- warps ----------

export interface Banner {
  gachaId: number
  begin: number
  end: number
  featured: number[]
  prize: number[]
}

export interface Pull {
  /** record id: unique, grows with time */
  id: string
  /** 1 standard, 2 departure, 11 character event, 12 light cone event, 21/22 collab */
  gachaType: number
  gachaId: number
  itemId: number
  rank: number
  itemType: string
  name: string
  /** server local time "YYYY-MM-DD HH:MM:SS" */
  time: string
}

export interface LivePull {
  gachaId: number
  itemId: number
  isNew: boolean
  at: number
}

export interface Gacha {
  banners: Banner[]
  /** record id -> pull */
  history: Record<string, Pull>
  importedAt: number | null
  /** pulls seen in the traffic, until the history import covers them */
  live: LivePull[]
}

const readGacha = () => invoke<Versioned<Gacha>>('get_gacha')

export function useGacha(): Gacha | null {
  return useLive(readGacha, { 'gacha-changed': () => 'refetch' })
}

/**
 * Downloads the warp history through the official API. The player must have
 * opened the warp history in game within ~24 h (Nous reads the key from the
 * game's web cache). Resolves to the number of new pulls; rejects with a
 * message to show.
 */
export function importGachaHistory(): Promise<number> {
  return invoke('import_gacha_history')
}

// ---------- raw snapshots ----------

export interface Snapshot {
  at: number
  /** protobuf message as JSON; some field names are obfuscated */
  data: unknown
}

/** [command name, what it holds] */
export function getSnapshotKinds(): Promise<[string, string][]> {
  return invoke('get_snapshot_kinds')
}

/** Latest raw answer of the game, e.g. useSnapshot('GetDailyActiveInfoScRsp'). */
export function useSnapshot(name: string): Snapshot | null {
  const read = useCallback(() => invoke<Versioned<Snapshot | null>>('get_snapshot', { name }), [name])
  return useLive(read, {
    'snapshot-changed': (current: Snapshot | null, changed: string) => (changed === name ? 'refetch' : current),
  })
}
