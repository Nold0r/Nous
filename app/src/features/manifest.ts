import type { ComponentType } from 'react'
import { parse } from 'smol-toml'
import type { IconSource } from '../ui'
import { categories, type CategoryId } from './categories.ts'
import type { ModuleDef, ModuleKind, ScreenDef } from './types'

/** modules/<id>/module.toml: how the app shows the module. */
interface Manifest {
  module: { id: string; title: string; kind: ModuleKind; badge?: string; dev_only?: boolean }
  /** Only for kind = "tab": the sidebar entry. */
  tab?: { icon?: string; category?: string; order?: number; default?: boolean }
}
/** What modules/<id>/ui.tsx may export. */
export interface UiExports {
  /** The screen (tab) or the Settings section (settings). */
  default?: ComponentType
  icon?: IconSource
  start?: ModuleDef['start']
  overlay?: ComponentType
  settings?: ModuleDef['settings']
}

const ID = /^[a-z0-9]+(-[a-z0-9]+)*$/
const categoryIds = new Set<string>(categories.map((c) => c.id))

/** Builds a module from its manifest and ui.tsx exports; throws a readable error. */
export function loadModule(path: string, raw: string, ui: UiExports): ModuleDef {
  const folder = path.split('/').at(-2)!
  const fail = (why: string): never => { throw new Error(`modules/${folder}/module.toml: ${why}`) }
  let manifest: Manifest
  try { manifest = parse(raw) as unknown as Manifest } catch (e) { return fail(String(e)) }
  const m = manifest.module ?? fail('[module] section is required')
  if (m.id !== folder || !ID.test(m.id)) fail('[module] id must match the folder name (kebab-case)')
  if (typeof m.title !== 'string' || !m.title) fail('[module] title is required')
  if (m.kind !== 'tab' && m.kind !== 'settings') fail('[module] kind must be "tab" or "settings"')
  if (!ui.default) fail('ui.tsx must export default the screen (tab) or Settings section (settings)')
  const module: ModuleDef = {
    id: m.id, title: m.title, kind: m.kind, devOnly: m.dev_only,
    start: ui.start, overlay: ui.overlay, settings: ui.settings,
  }
  if (m.kind === 'settings') {
    if (manifest.tab) fail('[tab] is only for kind = "tab"')
    return { ...module, section: ui.default }
  }
  const tab = manifest.tab ?? fail('[tab] section is required for kind = "tab"')
  if (!tab.category || !categoryIds.has(tab.category)) fail(`[tab] category must be one of: ${[...categoryIds].join(', ')}`)
  const icon = ui.icon ?? tab.icon ?? fail('[tab] icon is required (or export icon from ui.tsx)')
  const screen: ScreenDef = {
    id: m.id, title: m.title, icon, category: tab.category as CategoryId, order: tab.order,
    isDefault: tab.default, badge: m.badge, devOnly: m.dev_only, component: ui.default!,
  }
  return { ...module, screens: [screen] }
}
