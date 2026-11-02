import type { ComponentType } from 'react'
import type { IconSource } from '../ui'
import type { CategoryId } from './categories'

/** A sidebar section. Features are grouped under these headers. */
export interface CategoryDef {
  id: string
  /** Header shown above the group; omit for a group without a header. */
  title?: string
  /** Lower comes first. */
  order: number
  /** Stick the group to the bottom of the sidebar (used by settings). */
  pinBottom?: boolean
}

export interface ScreenDef {
  /** Unique kebab-case id, also the route: `/#/<id>`. */
  id: string
  title: string
  /** Game menu icon name from assets/icons/sign (see scripts/assets.json), or a lucide icon. */
  icon: IconSource
  /** Sidebar section, one of the ids in categories.ts. */
  category: CategoryId
  /** Order inside the category, lower comes first. */
  order?: number
  /**
   * Screen component. It's mounted at `/<id>/*`, so it may declare its own
   * nested <Routes> for sub-pages.
   */
  component: ComponentType
  /** Opened on start. Only one feature should set it. */
  isDefault?: boolean
  /** Small label next to the title in the sidebar, e.g. "beta". */
  badge?: string
  /** Only shown in a dev launch (`tauri dev`, `tauri build --debug`), hidden from release builds. */
  devOnly?: boolean
}

export function defineScreen(def: ScreenDef): ScreenDef {
  return def
}

export type ServiceId = 'hsr'
export type ModuleSettings = Record<string, unknown>
export interface ModuleStorage {
  get: (key: string) => unknown
  set: (key: string, value: unknown) => void
}
/** Capabilities supplied by the shell; background work must respect signal. */
export interface ModuleContext {
  signal: AbortSignal
  services: { hsr?: { read: <T>(command: string) => Promise<T> } }
  settings: ModuleSettings
  storage: ModuleStorage
  navigate: (screenId: string) => void
}
export type StopModule = () => void
/** A built-in module may expose zero, one or several screens. */
export interface ModuleDef {
  id: string
  title: string
  requires?: readonly ServiceId[]
  screens?: readonly ScreenDef[]
  devOnly?: boolean
  settings?: { defaults: ModuleSettings }
  /** Started once when enabled, independently of the current route. */
  start?: (context: ModuleContext) => StopModule | void
}
export function defineModule(def: ModuleDef): ModuleDef { return def }
