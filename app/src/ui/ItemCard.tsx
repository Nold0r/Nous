import type { ReactNode } from 'react'

/**
 * Card of a game item (relic, light cone…): picture on a rarity tile, title,
 * meta line, small status icons in the corner, then free content and a footer.
 * With `onClick` the whole card is a button.
 */
export function ItemCard({
  image,
  rarity,
  title,
  meta,
  badges,
  isNew,
  onClick,
  footer,
  children,
}: {
  /** picture URL */
  image: string
  rarity: number
  title: ReactNode
  /** line under the title, e.g. "Тело · +15" */
  meta?: ReactNode
  /** top-right corner: lock, marks */
  badges?: ReactNode
  /** recently obtained: gold «Новая» mark and a soft glow */
  isNew?: boolean
  onClick?: () => void
  /** bottom row separated by a line, e.g. owner or a note */
  footer?: ReactNode
  children?: ReactNode
}) {
  const r = Math.min(5, Math.max(1, rarity))
  const content = (
    <>
      <div className="ui-card__head">
        <span className={`ui-card__tile ui-rarity-tile`}>
          <img src={image} alt="" draggable={false} loading="lazy" />
        </span>
        <span className="ui-card__titles">
          <span className="ui-card__title">{title}</span>
          {meta && <span className="ui-card__meta">{meta}</span>}
        </span>
        {(badges || isNew) && (
          <span className="ui-card__badges">
            {isNew && <span className="ui-card__new">Новая</span>}
            {badges}
          </span>
        )}
      </div>
      {children && <div className="ui-card__body">{children}</div>}
      {footer && <div className="ui-card__footer">{footer}</div>}
    </>
  )
  const cls = `ui-card acrylic ui-edge ui-r${r}${onClick ? ' ui-lift' : ''}${isNew ? ' ui-card--new' : ''}`
  return onClick ? (
    <button type="button" className={cls} onClick={onClick}>
      {content}
    </button>
  ) : (
    <div className={cls}>{content}</div>
  )
}
