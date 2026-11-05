import type { ReactNode } from 'react'
import { Ban, Plus, X } from 'lucide-react'

/**
 * Filled chip for a chosen value. `include` is green with +, `exclude` red
 * with ⊘ (see FilterMenu), `neutral` plain. With `onRemove` it gets a × button.
 */
export function Tag({
  tone = 'neutral',
  onRemove,
  children,
}: {
  tone?: 'neutral' | 'include' | 'exclude'
  onRemove?: () => void
  children: ReactNode
}) {
  return (
    <span className={`ui-tag ui-tag--${tone}`}>
      {tone === 'include' && <Plus size={12} className="ui-tag__mark" />}
      {tone === 'exclude' && <Ban size={12} className="ui-tag__mark" />}
      <span className="ui-tag__text">{children}</span>
      {onRemove && (
        <button type="button" className="ui-tag__remove" onClick={onRemove} aria-label="Убрать">
          <X size={12} />
        </button>
      )}
    </span>
  )
}
