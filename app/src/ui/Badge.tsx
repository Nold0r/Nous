import type { ReactNode } from 'react'

export type Tone = 'neutral' | 'gold' | 'success' | 'warning' | 'danger' | 'info'

/** Small outlined label for statuses and tags. */
export function Badge({ tone = 'neutral', children }: { tone?: Tone; children: ReactNode }) {
  return <span className={`ui-badge ui-badge--${tone}`}>{children}</span>
}
