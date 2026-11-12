// Game icons downloaded by `npm run assets` (scripts/fetch-assets.mjs), keyed by file name.

type UrlMap = Record<string, string>

function byName(modules: Record<string, string>): UrlMap {
  return Object.fromEntries(
    Object.entries(modules).map(([file, url]) => [file.split('/').pop()!.replace(/\.png$/, ''), url]),
  )
}

// import.meta.glob options must be written inline (vite parses them statically)
/** Full-colour item icons (currencies, materials). */
export const itemIcons = byName(import.meta.glob<string>('../assets/icons/items/*.png', { eager: true, query: '?url', import: 'default' }))
/** White monochrome UI icons from the game menu, tinted via CSS mask. */
export const signIcons = byName(import.meta.glob<string>('../assets/icons/sign/*.png', { eager: true, query: '?url', import: 'default' }))
/** White stat icons (HP, crit rate, …), keyed like IconMaxHP; tinted via CSS mask. */
export const propertyIcons = byName(import.meta.glob<string>('../assets/icons/property/*.png', { eager: true, query: '?url', import: 'default' }))
/** Round player avatars. */
export const avatarIcons = byName(import.meta.glob<string>('../assets/icons/avatar/*.png', { eager: true, query: '?url', import: 'default' }))

export const DEFAULT_AVATAR = '8002'

const STAR_RAIL_RES = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/'

/**
 * Picture for a game head icon id (PlayerProfile.avatarId). Character portraits
 * are 20XXXX -> icon/avatar/XXXX.png, other profile pictures IconHead_<id>.png.
 * Bundled pictures are used when available, the rest load from StarRailRes.
 */
export function avatarUrl(headIconId: number | null): string {
  if (headIconId == null) return avatarIcons[DEFAULT_AVATAR]
  const character = headIconId >= 201000 && headIconId < 210000 ? String(headIconId - 200000) : null
  if (character && avatarIcons[character]) return avatarIcons[character]
  return STAR_RAIL_RES + `icon/avatar/${character ?? `IconHead_${headIconId}`}.png`
}
