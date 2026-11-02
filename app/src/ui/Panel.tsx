import type { ReactNode } from 'react'
import { SectionTitle } from './SectionTitle'

/**
 * Content container. `acrylic` (default) blurs the backdrop, `plain` is a flat
 * translucent fill for panels nested inside other panels.
 */
export function Panel({
  title,
  aside,
  variant = 'acrylic',
  compact,
  onClick,
  className,
  children,
}: {
  title?: ReactNode
  /** Right side of the title row (counter, button...). */
  aside?: ReactNode
  variant?: 'acrylic' | 'plain'
  compact?: boolean
  /** Makes the whole panel clickable (hover state, pointer). */
  onClick?: () => void
  className?: string
  children?: ReactNode
}) {
  const cls = [
    'ui-panel',
    variant === 'acrylic' ? 'acrylic' : 'ui-panel--plain',
    compact && 'ui-panel--compact',
    onClick && 'ui-panel--interactive',
    className,
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <section className={cls} onClick={onClick}>
      {title && <SectionTitle aside={aside}>{title}</SectionTitle>}
      {children}
    </section>
  )
}
