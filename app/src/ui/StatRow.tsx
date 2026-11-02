import type { ReactNode } from 'react'
import { ItemIcon } from './ItemIcon'

/**
 * One line of a stat list, styled like the character screen: icon, label,
 * value on the right, optional "/max". `icon` is an item icon name or any node.
 */
export function StatRow({
  icon,
  label,
  value,
  max,
  hint,
}: {
  icon?: string | ReactNode
  label: ReactNode
  value: ReactNode
  max?: ReactNode
  hint?: string
}) {
  return (
    <div className="stat-row" title={hint}>
      {icon != null && (
        <span className="stat-row__icon">{typeof icon === 'string' ? <ItemIcon name={icon} size={30} /> : icon}</span>
      )}
      <span className="stat-row__label">{label}</span>
      <span className="stat-row__value">
        {value}
        {max != null && <span className="stat-row__max">/{max}</span>}
      </span>
    </div>
  )
}

/** Vertical stack of StatRows with the standard gap. */
export function StatList({ children }: { children: ReactNode }) {
  return <div className="ui-stat-list">{children}</div>
}
