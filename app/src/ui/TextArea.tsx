import type { ReactNode } from 'react'

/**
 * Multi-line text field with a label, a hint and a character counter.
 * `onSubmit` runs on Ctrl+Enter. The caret starts at the end of the text.
 */
export function TextArea({
  label,
  value,
  onChange,
  onSubmit,
  placeholder,
  hint,
  maxLength,
  rows = 4,
  autoFocus,
}: {
  label?: ReactNode
  value: string
  onChange: (value: string) => void
  onSubmit?: () => void
  placeholder?: string
  hint?: ReactNode
  maxLength?: number
  rows?: number
  autoFocus?: boolean
}) {
  return (
    <label className="ui-textarea">
      {label && <span className="ui-textarea__label">{label}</span>}
      <textarea
        className="ui-textarea__input"
        rows={rows}
        maxLength={maxLength}
        placeholder={placeholder}
        autoFocus={autoFocus}
        value={value}
        onFocus={(e) => e.currentTarget.setSelectionRange(e.currentTarget.value.length, e.currentTarget.value.length)}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (onSubmit && e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault()
            onSubmit()
          }
        }}
      />
      {(hint || maxLength) && (
        <span className="ui-textarea__foot">
          <span>{hint}</span>
          {maxLength && (
            <span className="ui-textarea__count">
              {value.length} / {maxLength}
            </span>
          )}
        </span>
      )}
    </label>
  )
}
