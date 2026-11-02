import { invoke } from '@tauri-apps/api/core'
import { useLive, type Versioned } from './live'

// Mirrors src-tauri/src/inventory.rs. Values are raw game ids and rolls;
// names and stat values come from the game tables when shown.
// Docs: docs/Архитектура/Данные аккаунта.md

/** `cnt` rolls of a substat plus `step` extra value steps. */
export interface Affix {
  affixId: number
  cnt: number
  step: number
}

export interface Relic {
  /** The game's unique_id: stable while the relic exists (levelling, rerolls, equipping). Key notes by it. */
  uid: number
  /** RelicConfig id: set, slot, rarity */
  tid: number
  level: number
  exp: number
  mainAffixId: number
  subAffixes: Affix[]
  /** Variable Dice result waiting to be kept or discarded */
  reforgeSubAffixes?: Affix[]
  previewSubAffixes?: Affix[]
  /** path avatar id of the character wearing it */
  equippedBy: number | null
  locked: boolean
  discarded: boolean
  /** unix seconds Nous first saw it */
  firstSeen: number
  /** unix seconds of the last change Nous saw */
  updatedAt: number
}

export interface LightCone {
  uid: number
  tid: number
  level: number
  exp: number
  ascension: number
  superimposition: number
  equippedBy: number | null
  locked: boolean
}

export interface Material {
  tid: number
  count: number
  expireTime?: number
}

export interface Avatar {
  baseAvatarId: number
  level: number
  exp: number
  ascension: number
  currentPath: number
}

export interface AvatarPath {
  avatarId: number
  eidolon: number
  skillTree: Record<string, number>
  skillTreeVersion: number
  lightCone: number | null
  /** relic slot (1 head … 6 rope) -> relic uid */
  relics: Record<string, number>
}

export interface Inventory {
  // JSON object keys are strings: relics["17054"]
  relics: Record<string, Relic>
  lightCones: Record<string, LightCone>
  materials: Record<string, Material>
  avatars: Record<string, Avatar>
  avatarPaths: Record<string, AvatarPath>
  /** unix seconds of the last full list from the game, null if never */
  syncedAt: number | null
}

/** Payload of the `inventory-changed` event. */
export type InventoryEvent =
  | { kind: 'reset' }
  | {
      kind: 'changed'
      relics: Relic[]
      deletedRelics: number[]
      lightCones: boolean
      materials: boolean
      characters: boolean
    }

const readInventory = () => invoke<Versioned<Inventory>>('get_inventory')

export function getInventory(): Promise<Inventory> {
  return readInventory().then((v) => v.data)
}

export function getRelics(): Promise<Relic[]> {
  return invoke<Versioned<Relic[]>>('get_relics').then((v) => v.data)
}

/**
 * Inventory of the current account, kept live: relic changes are applied in
 * place, anything else reads again (see lib/live.ts for ordering).
 */
export function useInventory(): Inventory | null {
  return useLive(readInventory, {
    'inventory-changed': (inv: Inventory, e: InventoryEvent) => {
      if (e.kind === 'reset' || e.lightCones || e.materials || e.characters) return 'refetch'
      const relics = { ...inv.relics }
      for (const r of e.relics) relics[r.uid] = r
      for (const uid of e.deletedRelics) delete relics[uid]
      return { ...inv, relics }
    },
  })
}
