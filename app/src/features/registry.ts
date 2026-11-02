import { categories } from './categories'
import settings from './settings'
import type { CategoryDef, ModuleDef, ScreenDef } from './types'

/** Dev server or Tauri debug build. User mode deliberately ignores dev choices. */
export const isDevLaunch =
  (import.meta.env.DEV || import.meta.env.TAURI_ENV_DEBUG === 'true') && import.meta.env.VITE_USER_MODE !== 'true'

// Settings belongs to the shell and cannot be disabled.
const entries = import.meta.glob<{ default: ModuleDef }>('./*/index.ts', { eager: true })
export const modules: ModuleDef[] = Object.entries(entries)
  .filter(([path]) => path !== './settings/index.ts')
  .map(([, entry]) => entry.default)
  .filter((module) => !module.devOnly || isDevLaunch)

const seen = new Set<string>([settings.id])
const moduleIds = new Set<string>()
for (const module of modules) {
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(module.id) || moduleIds.has(module.id)) {
    throw new Error(`Invalid or duplicate module id: ${module.id}`)
  }
  moduleIds.add(module.id)
  for (const screen of module.screens ?? []) {
    if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(screen.id) || seen.has(screen.id)) {
      throw new Error(`Invalid or duplicate screen id: ${screen.id}`)
    }
    seen.add(screen.id)
  }
}
if (modules.flatMap((m) => m.screens ?? []).filter((s) => s.isDefault).length > 1) {
  throw new Error('Only one screen may be the default')
}
export function enabledScreens(enabled: ReadonlySet<string>): ScreenDef[] {
  return [...modules.filter((m) => enabled.has(m.id)).flatMap((m) => m.screens ?? []), settings]
    .filter((s) => !s.devOnly || isDevLaunch)
    .sort((a, b) => (a.order ?? 0) - (b.order ?? 0))
}
export interface FeatureGroup { category: CategoryDef; features: ScreenDef[] }
export function groupFeatures(features: ScreenDef[], pinBottom: boolean): FeatureGroup[] {
  return [...categories].filter((c: CategoryDef) => !!c.pinBottom === pinBottom).sort((a, b) => a.order - b.order)
    .map((category) => ({ category, features: features.filter((f) => f.category === category.id) }))
    .filter((group) => group.features.length > 0)
}
