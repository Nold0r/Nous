import { createContext, useContext } from 'react'
import type { ModuleSettings, ScreenDef } from './types'
interface ModuleState {
  ready: boolean
  saving: boolean
  enabled: ReadonlySet<string>
  screens: ScreenDef[]
  error: string | null
  setEnabled: (id: string, enabled: boolean) => void
  setSettings: (id: string, settings: ModuleSettings) => void
}
export const ModulesContext = createContext<ModuleState | null>(null)
export function useModules() {
  const state = useContext(ModulesContext)
  if (!state) throw new Error('ModuleProvider is missing')
  return state
}
