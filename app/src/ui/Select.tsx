/** Dropdown in the pill style of the design system (native <select> underneath). */
export function Select<T extends string | number>({
  value,
  options,
  onChange,
  title,
}: {
  value: T
  options: readonly { value: T; label: string }[]
  onChange: (value: T) => void
  title?: string
}) {
  return (
    <select
      className="ui-select"
      title={title}
      value={String(value)}
      onChange={(e) => {
        const opt = options.find((o) => String(o.value) === e.target.value)
        if (opt) onChange(opt.value)
      }}
    >
      {options.map((o) => (
        <option key={String(o.value)} value={String(o.value)}>
          {o.label}
        </option>
      ))}
    </select>
  )
}
