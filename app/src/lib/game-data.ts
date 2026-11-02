import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

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

/** Location and world progress; the player position updates live. */
export function useWorld(): World | null {
  const [world, setWorld] = useState<World | null>(null)
  useEffect(() => {
    if (!isTauri()) return
    invoke<World>('get_world').then(setWorld).catch(console.error)
    const offWorld = listen<World>('world-changed', (e) => setWorld(e.payload))
    const offMove = listen<Position | null>('player-moved', (e) =>
      setWorld((w) => (w ? { ...w, player: e.payload } : w)),
    )
    return () => {
      offWorld.then((f) => f())
      offMove.then((f) => f())
    }
  }, [])
  return world
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
  avatars: BattleAvatar[]
  monsters: BattleMonster[]
}

/** Battle reports, newest last; a new one arrives as soon as a fight ends. */
export function useBattles(): Battle[] {
  const [battles, setBattles] = useState<Battle[]>([])
  useEffect(() => {
    if (!isTauri()) return
    invoke<Battle[]>('get_battles').then(setBattles).catch(console.error)
    const unlisten = listen<Battle>('battle-finished', (e) => setBattles((b) => [...b, e.payload]))
    return () => {
      unlisten.then((f) => f())
    }
  }, [])
  return battles
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

export function useGacha(): Gacha | null {
  const [gacha, setGacha] = useState<Gacha | null>(null)
  useEffect(() => {
    if (!isTauri()) return
    const refetch = () => invoke<Gacha>('get_gacha').then(setGacha).catch(console.error)
    refetch()
    const unlisten = listen('gacha-changed', refetch)
    return () => {
      unlisten.then((f) => f())
    }
  }, [])
  return gacha
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
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null)
  useEffect(() => {
    if (!isTauri()) return
    const refetch = () => invoke<Snapshot | null>('get_snapshot', { name }).then(setSnapshot).catch(console.error)
    refetch()
    const unlisten = listen<string>('snapshot-changed', (e) => e.payload === name && refetch())
    return () => {
      unlisten.then((f) => f())
    }
  }, [name])
  return snapshot
}
