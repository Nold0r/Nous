import type { ReactNode } from 'react'

export type Tone = 'neutral' | 'gold' | 'success' | 'warning' | 'danger' | 'info'

/**
 * Small label for statuses and tags: outlined by default, `solid` is filled
 * (counters over pictures: E2, S5, «Новая»).
 */
export function Badge({ tone = 'neutral', solid, children }: { tone?: Tone; solid?: boolean; children: ReactNode }) {
  return <span className={`ui-badge ui-badge--${tone}${solid ? ' ui-badge--solid' : ''}`}>{children}</span>
}
