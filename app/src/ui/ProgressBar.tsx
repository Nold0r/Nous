import type { ReactNode } from 'react'
import { formatNumber } from '../lib/format'

/** Thin progress bar with an optional label and "value/max" on top. */
export function ProgressBar({
  value,
  max,
  label,
  tone = 'default',
}: {
  value: number
  max: number
  label?: ReactNode
  tone?: 'default' | 'gold' | 'success' | 'danger'
}) {
  const pct = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0
  return (
    <div className={`ui-progress${tone !== 'default' ? ` ui-progress--${tone}` : ''}`}>
      {label != null && (
        <div className="ui-progress__head">
          <span>{label}</span>
          <span>
            <span className="ui-progress__value">{formatNumber(value)}</span>/{formatNumber(max)}
          </span>
        </div>
      )}
      <div className="ui-progress__track">
        <div className="ui-progress__fill" style={{ width: `${pct}%` }} />
      </div>
    </div>
  )
}
