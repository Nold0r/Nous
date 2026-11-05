import type { ReactNode } from 'react'

/**
 * A character's card: the same panel, rarity line and hover as ItemCard,
 * with a square bust (corner marks over it), title, a line under it and a
 * footer row (level, light cone). With `onClick` the card is a button.
 */
export function PortraitCard({
  image,
  rarity,
  title,
  subtitle,
  topLeft,
  topRight,
  footer,
  dimmed,
  onClick,
}: {
  image: string
  rarity: number
  title: ReactNode
  subtitle?: ReactNode
  /** over the bust, e.g. element and path icons */
  topLeft?: ReactNode
  /** over the bust, e.g. eidolon badge */
  topRight?: ReactNode
  /** bottom row under a line, e.g. level and light cone */
  footer?: ReactNode
  /** faded, e.g. a path not in use */
  dimmed?: boolean
  onClick?: () => void
}) {
  const r = Math.min(5, Math.max(1, rarity))
  const content = (
    <>
      <span className="ui-portrait__frame ui-rarity-tile">
        <img src={image} alt="" draggable={false} loading="lazy" />
        {topLeft && <span className="ui-portrait__corner ui-portrait__corner--left">{topLeft}</span>}
        {topRight && <span className="ui-portrait__corner ui-portrait__corner--right">{topRight}</span>}
      </span>
      <span className="ui-portrait__titles">
        <span className="ui-portrait__title">{title}</span>
        {subtitle && <span className="ui-portrait__subtitle">{subtitle}</span>}
      </span>
      {footer && <span className="ui-card__footer">{footer}</span>}
    </>
  )
  const cls = `ui-card ui-portrait acrylic ui-edge ui-r${r}${onClick ? ' ui-lift' : ''}${dimmed ? ' ui-portrait--dimmed' : ''}`
  return onClick ? (
    <button type="button" className={cls} onClick={onClick}>
      {content}
    </button>
  ) : (
    <div className={cls}>{content}</div>
  )
}
