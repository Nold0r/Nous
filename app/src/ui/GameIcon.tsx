import type { LucideIcon } from 'lucide-react'
import { signIcons } from '../lib/icons'

/** A game menu icon name (file in assets/icons/sign) or a lucide icon as a fallback. */
export type IconSource = string | LucideIcon

/**
 * Monochrome game UI icon. The PNG is used as a mask, so the icon takes the
 * current text colour like a font glyph and follows hover/active states.
 */
export function GameIcon({ icon, size = 20, className }: { icon: IconSource; size?: number; className?: string }) {
  if (typeof icon !== 'string') {
    const Lucide = icon
    return <Lucide size={size} className={className} />
  }

  const url = signIcons[icon]
  if (!url) console.warn(`[icons] unknown sign icon "${icon}"`)
  return (
    <span
      className={`game-icon${className ? ` ${className}` : ''}`}
      style={{ width: size, height: size, maskImage: `url(${url})`, WebkitMaskImage: `url(${url})` }}
      aria-hidden
    />
  )
}
