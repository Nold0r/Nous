import type { ComponentType } from 'react'
import { parse } from 'smol-toml'
import type { IconSource } from '../ui'
import { categories, type CategoryId } from './categories.ts'
import type { ModuleDef, ModuleKind, ScreenDef, ServiceId } from './types'

/** modules/<id>/module.toml: how the app shows the module. */
interface Manifest {
  id: string
  title: string
  kind: ModuleKind
  icon?: string
  category?: string
  order?: number
  default?: boolean
  badge?: string
  dev_only?: boolean
  requires?: ServiceId[]
}
/** What modules/<id>/ui.tsx may export. */
export interface UiExports {
  /** The screen (tab) or the Settings section (plugin). */
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
  let m: Manifest
  try { m = parse(raw) as unknown as Manifest } catch (e) { return fail(String(e)) }
  if (m.id !== folder || !ID.test(m.id)) fail('id must match the folder name (kebab-case)')
  if (typeof m.title !== 'string' || !m.title) fail('title is required')
  if (m.kind !== 'tab' && m.kind !== 'plugin') fail('kind must be "tab" or "plugin"')
  if (m.requires?.some((s) => s !== 'hsr')) fail('requires may only list "hsr"')
  if (!ui.default) fail('ui.tsx must export default the screen (tab) or Settings section (plugin)')
  const module: ModuleDef = {
    id: m.id, title: m.title, kind: m.kind, requires: m.requires ?? [], devOnly: m.dev_only,
    start: ui.start, overlay: ui.overlay, settings: ui.settings,
  }
  if (m.kind === 'plugin') return { ...module, section: ui.default }
  if (!m.category || !categoryIds.has(m.category)) fail(`category must be one of: ${[...categoryIds].join(', ')}`)
  const icon = ui.icon ?? m.icon ?? fail('icon is required for a tab (in module.toml or exported from ui.tsx)')
  const screen: ScreenDef = {
    id: m.id, title: m.title, icon, category: m.category as CategoryId, order: m.order,
    isDefault: m.default, badge: m.badge, devOnly: m.dev_only, component: ui.default!,
  }
  return { ...module, screens: [screen] }
}
