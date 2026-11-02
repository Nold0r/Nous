import { invoke } from '@tauri-apps/api/core'
import { useLive, type Versioned } from './live'

// Mirrors `PlayerProfile` in src-tauri/src/profile.rs
export interface PlayerProfile {
  /** live: read from the game now; cached: saved from an earlier session; none: no account yet */
  source: 'live' | 'cached' | 'none'
  nickname: string | null
  uid: number | null
  level: number | null
  /** Head icon id from the game, see avatarUrl() in lib/icons.ts */
  avatarId: number | null
  stellarJade: number | null
  credits: number | null
  trailblazePower: number | null
  trailblazePowerMax: number | null
  reservedTrailblazePower: number | null
  reservedTrailblazePowerMax: number | null
  updatedAt: number | null
}

const empty: PlayerProfile = {
  source: 'none',
  nickname: null,
  uid: null,
  level: null,
  avatarId: null,
  stellarJade: null,
  credits: null,
  trailblazePower: null,
  trailblazePowerMax: 300,
  reservedTrailblazePower: null,
  reservedTrailblazePowerMax: 2400,
  updatedAt: null,
}

const readProfile = () => invoke<Versioned<PlayerProfile>>('get_profile')

/** Current profile; updates live while the game is running (event `profile-changed`). */
export function useProfile(): PlayerProfile {
  return (
    useLive(readProfile, {
      'profile-changed': (_: PlayerProfile, next: PlayerProfile) => next,
    }) ?? empty
  )
}

export { formatNumber } from './format'
