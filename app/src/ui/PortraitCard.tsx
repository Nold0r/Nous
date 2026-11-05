import type { ReactNode } from 'react'

/**
 * Card with a square picture (a character's bust) on a rarity background,
 * like the game's character list: corner marks over the picture, title and a line
 * under it. With `onClick` the card is a button.
 */
export function PortraitCard({
  image,
  rarity,
  title,
  subtitle,
  topLeft,
  topRight,
  bottom,
  dimmed,
  onClick,
}: {
  image: string
  rarity: number
  title: ReactNode
  subtitle?: ReactNode
  /** over the picture, e.g. element icon */
  topLeft?: ReactNode
  /** over the picture, e.g. eidolon badge */
  topRight?: ReactNode
  /** over the bottom of the picture, e.g. level */
  bottom?: ReactNode
  /** faded, e.g. a path not in use */
  dimmed?: boolean
  onClick?: () => void
}) {
  const r = Math.min(5, Math.max(1, rarity))
  const content = (
    <>
      <span className={`ui-portrait__frame ui-rarity-tile ui-edge-bottom ui-r${r}`}>
        <img src={image} alt="" draggable={false} loading="lazy" />
        {topLeft && <span className="ui-portrait__corner ui-portrait__corner--left">{topLeft}</span>}
        {topRight && <span className="ui-portrait__corner ui-portrait__corner--right">{topRight}</span>}
        {bottom && <span className="ui-portrait__bottom">{bottom}</span>}
      </span>
      <span className="ui-portrait__title">{title}</span>
      {subtitle && <span className="ui-portrait__subtitle">{subtitle}</span>}
    </>
  )
  const cls = `ui-portrait${onClick ? ' ui-portrait--interactive' : ''}${dimmed ? ' ui-portrait--dimmed' : ''}`
  return onClick ? (
    <button type="button" className={cls} onClick={onClick}>
      {content}
    </button>
  ) : (
    <div className={cls}>{content}</div>
  )
}
