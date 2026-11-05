import { useEffect, useRef, useState } from 'react'

/**
 * Open state of a dropdown anchored in `ref`: a click outside or Esc closes
 * it, Esc also returns focus to the trigger.
 */
export function usePopover<T extends HTMLElement>() {
  const ref = useRef<T>(null)
  const [open, setOpen] = useState(false)

  useEffect(() => {
    if (!open) return
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      e.stopPropagation()
      setOpen(false)
      ref.current?.querySelector<HTMLElement>('[data-popover-trigger]')?.focus()
    }
    document.addEventListener('mousedown', onDown)
    document.addEventListener('keydown', onKey, true)
    return () => {
      document.removeEventListener('mousedown', onDown)
      document.removeEventListener('keydown', onKey, true)
    }
  }, [open])

  return { ref, open, setOpen }
}
