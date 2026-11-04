import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Optional live damage through the Veritas plugin. It INJECTS INTO THE GAME
// and is off by default; see src-tauri/src/veritas.rs and
// docs/Архитектура/Veritas — урон в реальном времени.md.

export interface VeritasStatus {
  enabled: boolean
  version: string | null
  /** game versions the installed plugin supports, e.g. "4.3.54 - 4.6.52" */
  gameRange: string | null
  gameVersion: string | null
  /** null when unknown */
  compatible: boolean | null
  /** connected to the plugin's server (game running with Veritas loaded) */
  connected: boolean
}

/** Packet from Veritas: OnDamage, OnTurnBegin, OnTurnEnd, OnUseSkill, OnBattleEnd, … */
export interface VeritasPacket {
  name: string
  data: unknown
}

export function getVeritasStatus(): Promise<VeritasStatus> {
  return invoke('get_veritas_status')
}

/** Puts Veritas into the game folder. Only after the player accepted the risk. */
export function enableVeritas(acceptRisk: boolean): Promise<VeritasStatus> {
  return invoke('enable_veritas', { acceptRisk })
}

/** Removes Veritas from the game folder. */
export function disableVeritas(): Promise<VeritasStatus> {
  return invoke('disable_veritas')
}

export function useVeritasStatus(): VeritasStatus | null {
  const [status, setStatus] = useState<VeritasStatus | null>(null)
  useEffect(() => {
    if (!isTauri()) return
    const refetch = () => getVeritasStatus().then(setStatus).catch(console.error)
    refetch()
    const unlisten = listen('veritas-status', refetch)
    return () => {
      unlisten.then((f) => f())
    }
  }, [])
  return status
}

/** Calls `onPacket` for every live packet while Veritas is connected. */
export function useVeritasPackets(onPacket: (p: VeritasPacket) => void) {
  useEffect(() => {
    if (!isTauri()) return
    const unlisten = listen<VeritasPacket>('veritas', (e) => onPacket(e.payload))
    return () => {
      unlisten.then((f) => f())
    }
  }, [onPacket])
}
