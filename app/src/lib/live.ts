import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

// Keeps a piece of account state in sync with Rust. Reads and events carry the
// state revision (src-tauri game.rs `Versioned`), so:
//   1. subscribe first, then read;
//   2. events that arrive while the read is in flight are buffered and applied
//      only if newer than the read;
//   3. later events older than what we have are dropped;
//   4. `account-changed` (another account logged in) reads everything again.

export interface Versioned<T> {
  revision: number
  data: T
}

/** What an event does to the state: a new state, or 'refetch' to read again. */
export type Apply<T> = (state: T, data: never) => T | 'refetch'

export function useLive<T>(read: () => Promise<Versioned<T>>, events: Record<string, Apply<T>>, key = ''): T | null {
  const [state, setState] = useState<T | null>(null)
  // handlers may be inline functions: keep the latest without resubscribing
  const handlers = useRef(events)
  useEffect(() => {
    handlers.current = events
  })

  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false
    let current: T | null = null
    let revision = -1
    let buffer: { name: string; v: Versioned<unknown> }[] | null = []

    const apply = (name: string, v: Versioned<unknown>) => {
      if (v.revision <= revision || current === null) return
      const result = handlers.current[name](current, v.data as never)
      if (result === 'refetch') return sync()
      current = result
      revision = v.revision
      setState(current)
    }

    const sync = () => {
      buffer = buffer ?? []
      read()
        .then((v) => {
          if (cancelled) return
          if (v.revision >= revision) {
            current = v.data
            revision = v.revision
          }
          const pending = buffer ?? []
          buffer = null
          for (const e of pending) apply(e.name, e.v)
          setState(current)
        })
        .catch(console.error)
    }

    const onEvent = (name: string) => (e: { payload: Versioned<unknown> }) => {
      if (buffer) buffer.push({ name, v: e.payload })
      else apply(name, e.payload)
    }

    const subscriptions: Promise<UnlistenFn>[] = [
      ...Object.keys(handlers.current).map((name) => listen<Versioned<unknown>>(name, onEvent(name))),
      // another account: nothing we hold is valid any more
      listen('account-changed', () => {
        revision = -1
        sync()
      }),
    ]
    Promise.all(subscriptions).then(() => !cancelled && sync())

    return () => {
      cancelled = true
      subscriptions.forEach((s) => s.then((off) => off()))
    }
  }, [read, key])

  return state
}
