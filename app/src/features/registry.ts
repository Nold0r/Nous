import { categories } from './categories'
import { loadModule, type UiExports } from './manifest'
import settings from './settings'
import type { CategoryDef, ModuleDef, ScreenDef } from './types'

/** Dev server or Tauri debug build. User mode deliberately ignores dev choices. */
export const isDevLaunch =
  (import.meta.env.DEV || import.meta.env.TAURI_ENV_DEBUG === 'true') && import.meta.env.VITE_USER_MODE !== 'true'

// Modules live in app/modules/<id>/: module.toml says how the app shows the
// module, ui.tsx holds its screen (tab) or Settings section (settings). The shell
// lists none of them by name. Docs: docs/Архитектура/Модули.md
const manifests = import.meta.glob<string>('../../modules/*/module.toml', { eager: true, query: '?raw', import: 'default' })
const uis = import.meta.glob<UiExports>('../../modules/*/ui.tsx', { eager: true })
const load = (path: string, raw: string) => loadModule(path, raw, uis[path.replace(/module\.toml$/, 'ui.tsx')] ?? {})

export const modules: ModuleDef[] = Object.entries(manifests)
  .map(([path, raw]) => load(path, raw))
  .filter((module) => !module.devOnly || isDevLaunch)
  .sort((a, b) => a.id.localeCompare(b.id))

if (new Set(modules.map((m) => m.id)).size !== modules.length || modules.some((m) => m.id === settings.id)) {
  throw new Error('Duplicate module id')
}
if (modules.flatMap((m) => m.screens ?? []).filter((s) => s.isDefault).length > 1) {
  throw new Error('Only one screen may be the default')
}
export function enabledScreens(enabled: ReadonlySet<string>): ScreenDef[] {
  return [...modules.filter((m) => enabled.has(m.id)).flatMap((m) => m.screens ?? []), settings]
    .filter((s) => !s.devOnly || isDevLaunch)
    .sort((a, b) => (a.order ?? 0) - (b.order ?? 0))
}
/** Enabled settings-kind modules, for their sections on the Settings page. */
export function enabledSections(enabled: ReadonlySet<string>): ModuleDef[] {
  return modules.filter((m) => m.kind === 'settings' && m.section && enabled.has(m.id))
}
/** `#/overlay/<id>/...`: a module's page in a window over the game. */
export function overlayModule(hash: string): ModuleDef | undefined {
  const id = /^#\/overlay\/([a-z0-9-]+)/.exec(hash)?.[1]
  return modules.find((m) => m.id === id && m.overlay)
}
export interface FeatureGroup { category: CategoryDef; features: ScreenDef[] }
export function groupFeatures(features: ScreenDef[], pinBottom: boolean): FeatureGroup[] {
  return [...categories].filter((c: CategoryDef) => !!c.pinBottom === pinBottom).sort((a, b) => a.order - b.order)
    .map((category) => ({ category, features: features.filter((f) => f.category === category.id) }))
    .filter((group) => group.features.length > 0)
}
