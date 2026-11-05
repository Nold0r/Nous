import type { ReactNode } from 'react'

/**
 * Frame for a large character art on a rarity background, the art filling
 * it (an <img>, or an animated art), `children` laid over the bottom (name,
 * level…). The art fades in once; any motion is the art's own.
 */
export function HeroArt({ art, rarity, children }: { art: ReactNode; rarity: number; children?: ReactNode }) {
  const r = Math.min(5, Math.max(1, rarity))
  return (
    <div className={`ui-hero ui-edge ui-r${r}`}>
      <span className="ui-hero__art">{art}</span>
      {children && <div className="ui-hero__overlay">{children}</div>}
    </div>
  )
}
