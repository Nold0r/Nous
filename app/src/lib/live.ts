import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { startLive, type Apply, type Versioned } from './live-core'

export type { Apply, Versioned }

/** Keeps a piece of account state in sync with Rust; the rules are in live-core.ts. */
export function useLive<T>(read: () => Promise<Versioned<T>>, events: Record<string, Apply<T>>, key = ''): T | null {
  const [state, setState] = useState<T | null>(null)
  // handlers may be inline functions: keep the latest without resubscribing
  const handlers = useRef(events)
  useEffect(() => {
    handlers.current = events
  })

  useEffect(() => {
    if (!isTauri()) return
    return startLive<T>({
      read,
      handlers: () => handlers.current,
      listen: (name, listener) => listen<Versioned<unknown>>(name, listener),
      show: setState,
    })
  }, [read, key])

  return state
}
