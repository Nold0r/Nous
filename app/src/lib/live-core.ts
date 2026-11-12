// The logic of useLive without React or Tauri, so it runs in a plain node test
// (scripts/tests/live.test.mjs). Reads and events carry the state revision
// (Rust `Versioned`), so:
//   1. subscribe first, then read;
//   2. events that arrive while a read is in flight are buffered and applied
//      only if newer than the read;
//   3. later events older than what we have are dropped;
//   4. `account-changed` (another account logged in) reads everything again,
//      and an answer that was already on its way is thrown away;
//   5. a failed read is retried: the buffered events are kept until it succeeds.

export interface Versioned<T> {
  revision: number
  data: T
}

/** What an event does to the state: a new state, or 'refetch' to read again. */
export type Apply<T> = (state: T, data: never) => T | 'refetch'

type Listener = (event: { payload: Versioned<unknown> }) => void

export interface LiveOptions<T> {
  read: () => Promise<Versioned<T>>
  /** called at every event: handlers may be replaced without resubscribing */
  handlers: () => Record<string, Apply<T>>
  listen: (name: string, listener: Listener) => Promise<() => void>
  show: (state: T) => void
  onError?: (error: unknown) => void
  retryMs?: number
}

/** Starts following a piece of state; returns the function that stops it. */
export function startLive<T>({ read, handlers, listen, show, onError = console.error, retryMs = 1000 }: LiveOptions<T>): () => void {
  let cancelled = false
  let current: T | undefined
  // `current` may legitimately be null (a snapshot not received yet)
  let loaded = false
  let revision = -1
  let buffer: { name: string; v: Versioned<unknown> }[] | null = []
  let reading = false
  // grows when what is being read stops being wanted (account switch)
  let generation = 0
  let retry: ReturnType<typeof setTimeout> | undefined

  const apply = (name: string, v: Versioned<unknown>) => {
    if (v.revision <= revision || !loaded) return
    const result = handlers()[name](current as T, v.data as never)
    if (result === 'refetch') return sync()
    current = result
    revision = v.revision
    show(current)
  }

  const sync = () => {
    buffer = buffer ?? []
    if (reading) return
    reading = true
    const wanted = generation
    read().then(
      (v) => {
        reading = false
        if (cancelled) return
        if (wanted !== generation) return sync()
        if (v.revision >= revision) {
          current = v.data
          revision = v.revision
          loaded = true
        }
        const pending = buffer ?? []
        buffer = null
        for (const e of pending) {
          apply(e.name, e.v)
          // an event asked to read again: that read covers the rest
          if (buffer) break
        }
        if (loaded && !buffer) show(current as T)
      },
      (error) => {
        reading = false
        if (cancelled) return
        onError(error)
        retry = setTimeout(sync, retryMs)
      },
    )
  }

  const onEvent = (name: string) => (e: { payload: Versioned<unknown> }) => {
    if (buffer) buffer.push({ name, v: e.payload })
    else apply(name, e.payload)
  }

  const subscriptions = [
    ...Object.keys(handlers()).map((name) => listen(name, onEvent(name))),
    // another account: nothing we hold or are reading is valid any more
    listen('account-changed', () => {
      revision = -1
      generation++
      sync()
    }),
  ]
  Promise.all(subscriptions).then(() => !cancelled && sync(), onError)

  return () => {
    cancelled = true
    clearTimeout(retry)
    subscriptions.forEach((s) => s.then((off) => off()).catch(() => {}))
  }
}
