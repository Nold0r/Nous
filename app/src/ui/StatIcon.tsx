import { propertyIcons } from '../lib/icons'

/**
 * Game stat icon (HP, crit rate, …) by file name, e.g. `<StatIcon icon="IconMaxHP" />`.
 * Used as a mask like GameIcon, so it takes the text colour. `title` names the
 * stat for a tooltip and screen readers.
 */
export function StatIcon({ icon, size = 18, title }: { icon: string; size?: number; title?: string }) {
  const url = propertyIcons[icon]
  if (!url) console.warn(`[icons] unknown stat icon "${icon}"`)
  return (
    <span
      className="game-icon ui-stat-icon"
      style={{ width: size, height: size, maskImage: `url(${url})`, WebkitMaskImage: `url(${url})` }}
      role={title ? 'img' : undefined}
      aria-label={title}
      aria-hidden={title ? undefined : true}
      title={title}
    />
  )
}
