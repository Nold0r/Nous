import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

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

/** Current profile; updates live while the game is running (event `profile-changed`). */
export function useProfile(): PlayerProfile {
  const [profile, setProfile] = useState<PlayerProfile>(empty)

  useEffect(() => {
    // plain `npm run dev` in a browser has no Rust backend
    if (!isTauri()) return
    invoke<PlayerProfile>('get_profile')
      .then(setProfile)
      .catch((e) => console.error('get_profile failed', e))
    const unlisten = listen<PlayerProfile>('profile-changed', (e) => setProfile(e.payload))
    return () => {
      unlisten.then((f) => f())
    }
  }, [])

  return profile
}

export { formatNumber } from './format'
