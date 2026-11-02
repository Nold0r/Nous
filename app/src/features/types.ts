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

/**
 * One screen of the app. Drop a folder with an `index.ts` exporting this as
 * default and it shows up in the sidebar.
 * Docs: docs/Архитектура/Регистрация экранов.md
 */
export interface FeatureDef {
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

export function defineFeature(def: FeatureDef): FeatureDef {
  return def
}
