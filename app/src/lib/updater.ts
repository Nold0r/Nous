import { useCallback, useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { isDevLaunch } from '../features/registry'

export type UpdateState =
  | { state: 'none' }
  | { state: 'available'; version: string; notes: string }
  | { state: 'installing'; percent: number | null }
  | { state: 'error'; message: string }

/**
 * Checks GitHub releases for a newer Nous once at startup (release builds only)
 * and installs it on request: download, run the installer, restart.
 */
export function useUpdater(): { update: UpdateState; install: () => void } {
  const [found, setFound] = useState<Update | null>(null)
  const [update, setUpdate] = useState<UpdateState>({ state: 'none' })

  useEffect(() => {
    if (!isTauri() || isDevLaunch) return
    check()
      .then((u) => {
        if (!u) return
        setFound(u)
        setUpdate({ state: 'available', version: u.version, notes: u.body ?? '' })
      })
      // offline or GitHub unavailable: try again next launch
      .catch((e) => console.warn('update check failed', e))
  }, [])

  const install = useCallback(() => {
    if (!found) return
    let total = 0
    let done = 0
    setUpdate({ state: 'installing', percent: null })
    found
      .download((e) => {
        if (e.event === 'Started') total = e.data.contentLength ?? 0
        if (e.event === 'Progress') {
          done += e.data.chunkLength
          setUpdate({ state: 'installing', percent: total ? Math.round((done / total) * 100) : null })
        }
      })
      // the installer ends the process at once, without the normal exit:
      // write the account and stop the modules first
      .then(() => invoke('prepare_update'))
      .then(() => found.install())
      .then(() => relaunch())
      .catch((e) => setUpdate({ state: 'error', message: String(e) }))
  }, [found])

  return { update, install }
}
