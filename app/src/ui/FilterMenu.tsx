import { useState, type ReactNode } from 'react'
import { Ban, ChevronDown, Plus, Search } from 'lucide-react'
import { usePopover } from './usePopover'
import { Tag } from './Tag'

/** include: show only matching items (white list), exclude: hide them (black list). */
export type FilterMode = 'include' | 'exclude'
/** option value -> mode; values not in the map are not filtered. */
export type FilterState = Record<string, FilterMode>

export interface FilterOption {
  value: string
  label: string
  icon?: ReactNode
}

/** Longer lists get a search field. */
const SEARCH_FROM = 10

const next = (mode: FilterMode | undefined): FilterMode | undefined =>
  mode === undefined ? 'include' : mode === 'include' ? 'exclude' : undefined

function set(state: FilterState, value: string, mode: FilterMode | undefined): FilterState {
  const copy = { ...state }
  if (mode) copy[value] = mode
  else delete copy[value]
  return copy
}

/**
 * Dropdown filter where every option can be included (green, white list) or
 * excluded (red, black list). Clicking a row cycles: off → include → exclude
 * → off; the + and ⊘ buttons set a mode directly. Show the chosen values with
 * <FilterTags>.
 */
export function FilterMenu({
  label,
  options,
  value,
  onChange,
}: {
  label: string
  options: readonly FilterOption[]
  value: FilterState
  onChange: (value: FilterState) => void
}) {
  const { ref, open, setOpen } = usePopover<HTMLDivElement>()
  const [query, setQuery] = useState('')
  const included = Object.values(value).filter((m) => m === 'include').length
  const excluded = Object.values(value).filter((m) => m === 'exclude').length
  const q = query.trim().toLowerCase()
  const shown = q ? options.filter((o) => o.label.toLowerCase().includes(q)) : options

  return (
    <div className="ui-dropdown" ref={ref}>
      <button
        type="button"
        className={`ui-select${open ? ' ui-select--open' : ''}${included + excluded ? ' ui-select--active' : ''}`}
        aria-haspopup="true"
        aria-expanded={open}
        data-popover-trigger
        onClick={() => setOpen(!open)}
      >
        <span className="ui-select__value">{label}</span>
        {included > 0 && <span className="ui-filter__count ui-filter__count--include">+{included}</span>}
        {excluded > 0 && <span className="ui-filter__count ui-filter__count--exclude">−{excluded}</span>}
        <ChevronDown size={14} className="ui-select__chevron" />
      </button>
      {open && (
        <div className="ui-popover ui-popover--filter">
          {options.length >= SEARCH_FROM && (
            <label className="ui-popover__search">
              <Search size={14} />
              <input autoFocus placeholder="Найти" value={query} onChange={(e) => setQuery(e.target.value)} />
            </label>
          )}
          <div className="ui-popover__hint">
            <span>
              <Plus size={12} /> показать только
            </span>
            <span>
              <Ban size={12} /> скрыть
            </span>
            {included + excluded > 0 && (
              <button type="button" className="ui-popover__reset" onClick={() => onChange({})}>
                Сбросить
              </button>
            )}
          </div>
          <div className="ui-popover__list">
            {shown.map((o) => {
              const mode = value[o.value]
              return (
                <div key={o.value} className={`ui-filter__row${mode ? ` ui-filter__row--${mode}` : ''}`}>
                  <button
                    type="button"
                    className="ui-filter__label"
                    aria-pressed={!!mode}
                    onClick={() => onChange(set(value, o.value, next(mode)))}
                  >
                    {o.icon}
                    <span>{o.label}</span>
                  </button>
                  <button
                    type="button"
                    className="ui-filter__mode ui-filter__mode--include"
                    title="Показать только"
                    aria-pressed={mode === 'include'}
                    onClick={() => onChange(set(value, o.value, mode === 'include' ? undefined : 'include'))}
                  >
                    <Plus size={14} />
                  </button>
                  <button
                    type="button"
                    className="ui-filter__mode ui-filter__mode--exclude"
                    title="Скрыть"
                    aria-pressed={mode === 'exclude'}
                    onClick={() => onChange(set(value, o.value, mode === 'exclude' ? undefined : 'exclude'))}
                  >
                    <Ban size={14} />
                  </button>
                </div>
              )
            })}
            {shown.length === 0 && <div className="ui-popover__empty">Ничего не найдено</div>}
          </div>
        </div>
      )}
    </div>
  )
}

/**
 * The values chosen in several FilterMenus as removable tags: green for
 * included, red for excluded, plus «Сбросить всё». Renders nothing when no
 * filter is set.
 */
export function FilterTags({
  groups,
  onReset,
}: {
  /** `tag`: short group name for the tags (default `label`), '' for none */
  groups: readonly {
    label: string
    tag?: string
    options: readonly FilterOption[]
    value: FilterState
    onChange: (v: FilterState) => void
  }[]
  onReset: () => void
}) {
  const tags = groups.flatMap((g) =>
    Object.entries(g.value).map(([value, mode]) => {
      const option = g.options.find((o) => o.value === value)
      return (
        <Tag key={`${g.label}:${value}`} tone={mode} onRemove={() => g.onChange(set(g.value, value, undefined))}>
          {option?.icon}
          {(g.tag ?? g.label) && <span className="ui-tag__group">{g.tag ?? g.label}:</span>}
          {option?.label ?? value}
        </Tag>
      )
    }),
  )
  if (tags.length === 0) return null
  return (
    <div className="ui-filter-tags">
      {tags}
      <button type="button" className="ui-filter-tags__reset" onClick={onReset}>
        Сбросить всё
      </button>
    </div>
  )
}
