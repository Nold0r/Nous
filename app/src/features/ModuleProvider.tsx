import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { enabledScreens, isDevLaunch, modules } from './registry'
import { ModuleRuntime } from './module-runtime'
import type { ModuleSettings } from './types'
import { ModulesContext } from './module-context'

const CONFIG_KEY = 'nous.modules.v1'
interface Configuration { disabled: string[]; settings: Record<string, ModuleSettings> }
function load(): Configuration {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(CONFIG_KEY) ?? 'null')
    if (saved && typeof saved === 'object' && 'disabled' in saved && Array.isArray(saved.disabled)) {
      return { disabled: saved.disabled.filter((id): id is string => typeof id === 'string'),
        settings: 'settings' in saved && saved.settings && typeof saved.settings === 'object'
          ? saved.settings as Record<string, ModuleSettings> : {} }
    }
  } catch { /* An unavailable or older preference store falls back to enabled modules. */ }
  return { disabled: [], settings: {} }
}
function storage(id: string) {
  const key = `nous.module.${id}.v1.`
  return {
    get: (name: string): unknown => JSON.parse(localStorage.getItem(key + name) ?? 'null'),
    set: (name: string, value: unknown) => localStorage.setItem(key + name, JSON.stringify(value)),
  }
}
function useModuleState() {
  const [configuration, setConfiguration] = useState(load)
  const [ready, setReady] = useState(() => !isTauri())
  const [saving, setSaving] = useState(false)
  const pending = useRef(false)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false
    invoke<Configuration | null>('get_module_configuration').then(async (saved) => {
      if (cancelled) return
      const next = saved ?? load()
      // Import existing WebView choices once, before starting any module work.
      if (!saved) await invoke('set_module_configuration', { configuration: next })
      if (!cancelled) { setConfiguration(next); setReady(true) }
    }).catch(() => {
      if (!cancelled) { setError('Не удалось прочитать настройки модулей. Проверьте доступ к папке данных.'); setReady(true) }
    })
    return () => { cancelled = true }
  }, [])
  // Release builds always use their built-in defaults, even with old dev preferences.
  const enabled = useMemo(() => new Set(modules.filter((m) => !isDevLaunch || !configuration.disabled.includes(m.id)).map((m) => m.id)), [configuration])
  const [runtime] = useState(() => new ModuleRuntime(
    async (services) => { if (isTauri()) await invoke('set_capture_enabled', { enabled: services.includes('hsr') }) },
    (module) => ({ storage: storage(module.id), navigate: (id) => { window.location.hash = `/${id}` },
      services: { hsr: module.requires?.includes('hsr') ? { read: <T,>(command: string) => invoke<T>(command) } : undefined } }),
    setError,
  ))
  useEffect(() => { if (ready) void runtime.update(modules.filter((m) => enabled.has(m.id)), configuration.settings) }, [runtime, enabled, configuration.settings, ready])
  useEffect(() => () => { void runtime.update([]) }, [runtime])
  const save = async (next: Configuration) => {
    if (pending.current) return
    pending.current = true
    setSaving(true)
    try {
      if (isTauri()) await invoke('set_module_configuration', { configuration: next })
      else localStorage.setItem(CONFIG_KEY, JSON.stringify(next))
      setConfiguration(next)
      setError(null)
    } catch { setError('Не удалось сохранить выбор модулей. Проверьте доступ к хранилищу и повторите.') }
    finally { pending.current = false; setSaving(false) }
  }
  return {
    ready, saving, enabled, screens: enabledScreens(enabled), error,
    setEnabled: (id: string, value: boolean) => {
      if (!isDevLaunch || !modules.some((m) => m.id === id)) return
      void save({ ...configuration, disabled: [...configuration.disabled.filter((item) => item !== id), ...(!value ? [id] : [])] })
    },
    setSettings: (id: string, settings: ModuleSettings) => {
      if (modules.some((m) => m.id === id)) void save({ ...configuration, settings: { ...configuration.settings, [id]: settings } })
    },
  }
}
export function ModuleProvider({ children }: { children: ReactNode }) {
  const state = useModuleState()
  return <ModulesContext.Provider value={state}>{children}</ModulesContext.Provider>
}
