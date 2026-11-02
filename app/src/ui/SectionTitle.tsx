import type { ReactNode } from 'react'

/** Heading of a block inside a page: gold diamond, text, fading line, optional right side. */
export function SectionTitle({ children, aside }: { children: ReactNode; aside?: ReactNode }) {
  return (
    <h2 className="ui-section-title">
      <span className="ui-section-title__mark" aria-hidden>
        ◆
      </span>
      <span className="ui-section-title__text">{children}</span>
      <span className="ui-section-title__line" />
      {aside && <span className="ui-section-title__aside">{aside}</span>}
    </h2>
  )
}
