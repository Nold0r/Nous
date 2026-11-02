import type { ReactNode } from 'react'
import { GameIcon, type IconSource } from './GameIcon'

/** Shown when a screen or block has nothing to display yet. */
export function EmptyState({
  icon,
  title,
  children,
  action,
}: {
  icon?: IconSource
  title: string
  children?: ReactNode
  /** Usually a <Button>. */
  action?: ReactNode
}) {
  return (
    <div className="ui-empty acrylic">
      {icon && <GameIcon icon={icon} size={44} className="ui-empty__icon" />}
      <div className="ui-empty__title">{title}</div>
      {children && <div className="ui-empty__text">{children}</div>}
      {action && <div className="ui-empty__action">{action}</div>}
    </div>
  )
}
