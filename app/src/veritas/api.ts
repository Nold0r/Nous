import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { defaultSettings } from './types'
import type { Battle, OverlaySettings, OverlayState, PluginConfig, PluginOptions } from './types'

const browserState: OverlayState = {
  settings: defaultSettings,
  snapshot: { revision: 0, connected: false, active: false, battle: null, history: [], diagnostics: [] },
  presentation: { targetFound: false, targetVisible: false, exclusiveFullscreen: false, visible: false, message: null },
  hidden: false, replay: false,
}

export const getBattle = (id: number) => invoke<Battle | null>('get_veritas_battle', { id })
export const exportBattle = (id: number, format: 'json' | 'csv' | 'both') =>
  invoke<{ directory: string; files: string[] }>('export_veritas_battle', { id, format })
export const overlayAction = (action: 'toggle' | 'show' | 'hide' | 'settings' | 'reset') =>
  invoke<OverlayState>('veritas_overlay_action', { action })
export const readPluginConfig = () => invoke<PluginConfig>('read_veritas_plugin_config')
export const writePluginConfig = (options: PluginOptions) => invoke<PluginConfig>('write_veritas_plugin_config', { options })
export const checkUpdate = () => invoke<{ version: string; notes: string; gameRange: string | null; prerelease: boolean }>('check_veritas_update')
/** The native adapter requires a previous explicit consent and owned installation. */
export const installUpdate = () => invoke('enable_veritas', { acceptRisk: true, updateOnly: true })

/** One subscription per surface; settings writes are serialized to preserve slider edits. */
export function useOverlay() {
  const [state, setState] = useState<OverlayState | null>(isTauri() ? null : browserState)
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const current = useRef(state)
  const queued = useRef<OverlaySettings | null>(null)
  const writing = useRef(false)
  const mounted = useRef(true)

  useEffect(() => {
    mounted.current = true
    if (!isTauri()) return () => { mounted.current = false }
    let alive = true
    let receivedEvent = false
    const apply = (next: OverlayState) => {
      if (!alive || !mounted.current) return
      const merged = writing.current && current.current ? { ...next, settings: current.current.settings } : next
      current.current = merged
      setState(merged)
    }
    const subscription = listen<OverlayState>('veritas-overlay', ({ payload }) => {
      receivedEvent = true
      apply(payload)
    })
    subscription.then(() => invoke<OverlayState>('get_veritas_overlay')).then((next) => {
      if (!receivedEvent) apply(next)
    }).catch((e) => { if (alive && mounted.current) setError(String(e)) })
    return () => {
      alive = false
      mounted.current = false
      void subscription.then((unlisten) => unlisten()).catch(() => {})
    }
  }, [])

  const patch = useCallback(async (changes: Partial<OverlaySettings>) => {
    if (!current.current) return
    const settings = { ...current.current.settings, ...changes }
    current.current = { ...current.current, settings }
    setState(current.current)
    if (!isTauri()) return
    queued.current = settings
    if (writing.current) return
    writing.current = true
    setSaving(true)
    setError(null)
    try {
      while (queued.current) {
        const next = queued.current
        queued.current = null
        const saved = await invoke<OverlayState>('set_veritas_overlay_settings', { settings: next })
        if (!queued.current && mounted.current) {
          current.current = saved
          setState(saved)
        }
      }
    } catch (e) {
      queued.current = null
      if (mounted.current) {
        setError(`Настройки не сохранены: ${String(e)}`)
        try {
          const actual = await invoke<OverlayState>('get_veritas_overlay')
          current.current = actual
          if (mounted.current) setState(actual)
        } catch { /* Preserve the last view while disconnected. */ }
      }
    } finally {
      writing.current = false
      if (mounted.current) setSaving(false)
    }
  }, [])

  return { state, patch, error, saving }
}
