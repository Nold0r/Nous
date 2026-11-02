import type { CSSProperties, ReactNode } from 'react'

/** Responsive card grid: as many columns of at least `min` px as fit. */
export function Grid({ min = 240, children }: { min?: number; children: ReactNode }) {
  return (
    <div className="ui-grid" style={{ '--ui-grid-min': `${min}px` } as CSSProperties}>
      {children}
    </div>
  )
}
