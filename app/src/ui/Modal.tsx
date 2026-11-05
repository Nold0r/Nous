import { useEffect, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { X } from 'lucide-react'

/**
 * Dialog over the whole window. Rendered into <body> (an acrylic parent's
 * backdrop-filter would trap a fixed child). Esc or a click outside closes it;
 * the body scrolls when the content is long.
 */
export function Modal({
  open,
  title,
  subtitle,
  size = 'md',
  actions,
  onClose,
  footer,
  children,
}: {
  open: boolean
  title: ReactNode
  /** Small line under the title. */
  subtitle?: ReactNode
  /** md: text dialogs; lg: wide, for two-column content (item details). */
  size?: 'md' | 'lg'
  /** Head row, left of the close button, e.g. a Pager. */
  actions?: ReactNode
  onClose: () => void
  /** Bottom row, e.g. actions. */
  footer?: ReactNode
  children?: ReactNode
}) {
  useEffect(() => {
    if (!open) return
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && onClose()
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [open, onClose])

  if (!open) return null
  return createPortal(
    <div className="ui-modal" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`ui-modal__window ui-modal__window--${size} acrylic`} role="dialog" aria-modal="true">
        <div className="ui-modal__head">
          <span className="ui-modal__titles">
            <span className="ui-modal__title">{title}</span>
            {subtitle && <span className="ui-modal__subtitle">{subtitle}</span>}
          </span>
          {actions && <span className="ui-modal__actions">{actions}</span>}
          <button type="button" className="ui-modal__close" onClick={onClose} aria-label="Закрыть">
            <X size={18} />
          </button>
        </div>
        <div className="ui-modal__body">{children}</div>
        {footer && <div className="ui-modal__footer">{footer}</div>}
      </div>
    </div>,
    document.body,
  )
}
