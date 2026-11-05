import { Search, X } from 'lucide-react'

/** Pill search field with a magnifier and a clear button. Esc clears it. */
export function SearchInput({
  value,
  onChange,
  placeholder = 'Поиск',
}: {
  value: string
  onChange: (value: string) => void
  placeholder?: string
}) {
  return (
    <label className="ui-search">
      <Search size={15} className="ui-search__icon" />
      <input
        className="ui-search__input"
        type="text"
        placeholder={placeholder}
        title={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => e.key === 'Escape' && value && onChange('')}
      />
      {value && (
        <button type="button" className="ui-search__clear" onClick={() => onChange('')} aria-label="Очистить">
          <X size={14} />
        </button>
      )}
    </label>
  )
}
