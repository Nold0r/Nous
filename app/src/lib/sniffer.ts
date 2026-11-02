import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// Mirrors `SnifferStatus` in src-tauri/src/sniffer.rs
export type SnifferStatus =
  | { state: 'starting' }
  | { state: 'needs_admin' }
  | { state: 'waiting_for_login' }
  | { state: 'connected' }
  | { state: 'error'; message: string }

/** State of the game traffic capture (event `sniffer-status`). */
export function useSnifferStatus(): SnifferStatus | null {
  const [status, setStatus] = useState<SnifferStatus | null>(null)

  useEffect(() => {
    if (!isTauri()) return
    invoke<SnifferStatus>('get_sniffer_status').then(setStatus).catch(console.error)
    const unlisten = listen<SnifferStatus>('sniffer-status', (e) => setStatus(e.payload))
    return () => {
      unlisten.then((f) => f())
    }
  }, [])

  return status
}

/** Relaunch Nous elevated (UAC prompt); the current window closes. */
export function restartAsAdmin(): Promise<void> {
  return invoke('restart_as_admin')
}
