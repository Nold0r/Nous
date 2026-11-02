import type { ReactNode } from 'react'

/**
 * On/off setting row: label (and an optional hint) on the left, a pill switch
 * on the right. The whole row is clickable.
 */
export function Switch({
  checked,
  onChange,
  disabled,
  label,
  hint,
}: {
  checked: boolean
  onChange: (checked: boolean) => void
  disabled?: boolean
  label: ReactNode
  hint?: ReactNode
}) {
  return (
    <label className={`ui-switch${disabled ? ' ui-switch--disabled' : ''}`}>
      <span className="ui-switch__text">
        <span className="ui-switch__label">{label}</span>
        {hint && <span className="ui-switch__hint">{hint}</span>}
      </span>
      <input
        type="checkbox"
        className="ui-switch__input"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span className="ui-switch__track" aria-hidden />
    </label>
  )
}
