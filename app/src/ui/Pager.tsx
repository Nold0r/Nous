import { useEffect } from 'react'
import { ChevronLeft, ChevronRight } from 'lucide-react'

/**
 * "‹ 3 / 6 ›" for paging through a short list (e.g. a character's relics in
 * a dialog). ←/→ page too, unless the focus is in a text field. Wraps around.
 */
export function Pager({ index, count, onChange }: { index: number; count: number; onChange: (index: number) => void }) {
  const go = (step: number) => onChange((index + step + count) % count)

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement
      if (el.closest('input, textarea, [contenteditable="true"]')) return
      if (e.key === 'ArrowLeft') onChange((index - 1 + count) % count)
      else if (e.key === 'ArrowRight') onChange((index + 1) % count)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [index, count, onChange])

  return (
    <span className="ui-pager">
      <button type="button" className="ui-pager__step" onClick={() => go(-1)} aria-label="Предыдущая" title="Предыдущая (←)">
        <ChevronLeft size={16} />
      </button>
      <span className="ui-pager__count">
        {index + 1} / {count}
      </span>
      <button type="button" className="ui-pager__step" onClick={() => go(1)} aria-label="Следующая" title="Следующая (→)">
        <ChevronRight size={16} />
      </button>
    </span>
  )
}
