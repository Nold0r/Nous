import type { ReactNode } from 'react'

/**
 * Frame of every feature screen: title row with optional actions, then content.
 * Children are stacked with the standard section gap.
 */
export function Page({
  title,
  subtitle,
  actions,
  children,
}: {
  title: string
  subtitle?: string
  /** Buttons on the right of the title row. */
  actions?: ReactNode
  children?: ReactNode
}) {
  return (
    <div className="page">
      <header className="page__header">
        <div className="page__titles">
          <h1>{title}</h1>
          {subtitle && <p>{subtitle}</p>}
        </div>
        {actions && <div className="page__actions">{actions}</div>}
      </header>
      <div className="page__body">{children}</div>
    </div>
  )
}
