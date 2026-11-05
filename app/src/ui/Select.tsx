import { useRef, type KeyboardEvent } from 'react'
import { Check, ChevronDown } from 'lucide-react'
import { usePopover } from './usePopover'

/**
 * Dropdown in the pill style of the design system: one value from a list.
 * Keyboard: Enter/Space/↓ opens, ↑/↓ move, Enter picks, Esc closes.
 */
export function Select<T extends string | number>({
  value,
  options,
  onChange,
  title,
}: {
  value: T
  options: readonly { value: T; label: string }[]
  onChange: (value: T) => void
  /** What the choice is about, shown as a tooltip and read by screen readers. */
  title?: string
}) {
  const { ref, open, setOpen } = usePopover<HTMLDivElement>()
  const list = useRef<HTMLDivElement>(null)
  const current = options.find((o) => o.value === value)

  const focusOption = (i: number) => list.current?.querySelectorAll<HTMLElement>('[role=option]')[i]?.focus()
  const openList = () => {
    setOpen(true)
    // after render: focus the chosen option
    requestAnimationFrame(() => focusOption(Math.max(0, options.findIndex((o) => o.value === value))))
  }
  const pick = (v: T) => {
    onChange(v)
    setOpen(false)
    ref.current?.querySelector<HTMLElement>('[data-popover-trigger]')?.focus()
  }

  const onTriggerKey = (e: KeyboardEvent) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault()
      openList()
    }
  }
  const onListKey = (e: KeyboardEvent) => {
    const items = [...(list.current?.querySelectorAll<HTMLElement>('[role=option]') ?? [])]
    const i = items.indexOf(document.activeElement as HTMLElement)
    if (e.key === 'ArrowDown') focusOption(Math.min(items.length - 1, i + 1))
    else if (e.key === 'ArrowUp') focusOption(Math.max(0, i - 1))
    else return
    e.preventDefault()
  }

  return (
    <div className="ui-dropdown" ref={ref}>
      <button
        type="button"
        className={`ui-select${open ? ' ui-select--open' : ''}`}
        title={title}
        aria-label={title ? `${title}: ${current?.label ?? ''}` : undefined}
        aria-haspopup="listbox"
        aria-expanded={open}
        data-popover-trigger
        onClick={() => (open ? setOpen(false) : openList())}
        onKeyDown={onTriggerKey}
      >
        <span className="ui-select__value">{current?.label ?? ''}</span>
        <ChevronDown size={14} className="ui-select__chevron" />
      </button>
      {open && (
        <div className="ui-popover" role="listbox" ref={list} onKeyDown={onListKey}>
          {options.map((o) => (
            <button
              key={String(o.value)}
              type="button"
              role="option"
              aria-selected={o.value === value}
              className={`ui-popover__item${o.value === value ? ' ui-popover__item--active' : ''}`}
              onClick={() => pick(o.value)}
            >
              <span>{o.label}</span>
              {o.value === value && <Check size={14} />}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}
