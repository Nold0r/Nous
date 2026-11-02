import { useEffect, useMemo, useState, type ReactNode } from 'react'
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
  const [error, setError] = useState<string | null>(null)
  // Release builds always use their built-in defaults, even with old dev preferences.
  const enabled = useMemo(() => new Set(modules.filter((m) => !isDevLaunch || !configuration.disabled.includes(m.id)).map((m) => m.id)), [configuration])
  const [runtime] = useState(() => new ModuleRuntime(
    async (services) => { if (isTauri()) await invoke('set_capture_enabled', { enabled: services.includes('hsr') }) },
    (module) => ({ storage: storage(module.id), navigate: (id) => { window.location.hash = `/${id}` },
      services: { hsr: module.requires?.includes('hsr') ? { read: <T,>(command: string) => invoke<T>(command) } : undefined } }),
    setError,
  ))
  useEffect(() => { void runtime.update(modules.filter((m) => enabled.has(m.id)), configuration.settings) }, [runtime, enabled, configuration.settings])
  useEffect(() => () => { void runtime.update([]) }, [runtime])
  const save = (next: Configuration) => {
    try { localStorage.setItem(CONFIG_KEY, JSON.stringify(next)); setConfiguration(next); setError(null) }
    catch { setError('Не удалось сохранить выбор модулей. Проверьте доступ к хранилищу и повторите.') }
  }
  return {
    enabled, screens: enabledScreens(enabled), error,
    setEnabled: (id: string, value: boolean) => {
      if (!isDevLaunch || !modules.some((m) => m.id === id)) return
      save({ ...configuration, disabled: [...configuration.disabled.filter((item) => item !== id), ...(!value ? [id] : [])] })
    },
    setSettings: (id: string, settings: ModuleSettings) => {
      if (modules.some((m) => m.id === id)) save({ ...configuration, settings: { ...configuration.settings, [id]: settings } })
    },
  }
}
export function ModuleProvider({ children }: { children: ReactNode }) {
  const state = useModuleState()
  return <ModulesContext.Provider value={state}>{children}</ModulesContext.Provider>
}
