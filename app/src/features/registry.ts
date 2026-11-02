import { categories } from './categories'
import type { CategoryDef, FeatureDef } from './types'

/**
 * Dev launch: Vite dev server, or any Tauri debug build (`tauri dev`,
 * `tauri build --debug`). Features with `devOnly` are shown only then.
 * `npm run tauri:user` turns it off to see the app as a user would.
 */
export const isDevLaunch =
  (import.meta.env.DEV || import.meta.env.TAURI_ENV_DEBUG === 'true') && import.meta.env.VITE_USER_MODE !== 'true'

// Every `features/<name>/index.ts` is picked up automatically.
const modules = import.meta.glob<{ default: FeatureDef }>('./*/index.ts', { eager: true })

export const features: FeatureDef[] = Object.entries(modules)
  .map(([file, m]) => {
    if (!m.default) console.error(`[features] ${file} has no default export (use defineFeature)`)
    return m.default
  })
  .filter((f) => f && (!f.devOnly || isDevLaunch))
  .sort((a, b) => (a.order ?? 0) - (b.order ?? 0))

// mistakes the type checker can't catch
const seen = new Set<string>()
for (const f of features) {
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(f.id)) console.error(`[features] id "${f.id}" must be kebab-case`)
  if (seen.has(f.id)) console.error(`[features] duplicate feature id "${f.id}"`)
  seen.add(f.id)
}
if (features.filter((f) => f.isDefault).length > 1) console.error('[features] more than one feature has isDefault')

export interface FeatureGroup {
  category: CategoryDef
  features: FeatureDef[]
}

/** Categories with their features, in sidebar order. Empty categories are skipped. */
export function groupFeatures(pinBottom: boolean): FeatureGroup[] {
  return [...categories]
    .filter((c: CategoryDef) => !!c.pinBottom === pinBottom)
    .sort((a, b) => a.order - b.order)
    .map((category) => ({ category, features: features.filter((f) => f.category === category.id) }))
    .filter((g) => g.features.length > 0)
}

export const defaultFeature: FeatureDef | undefined = features.find((f) => f.isDefault) ?? features[0]
