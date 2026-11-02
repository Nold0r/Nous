/** Segmented pill switch (like "Характеристики | Умения" in the game). */
export function Tabs<T extends string>({
  items,
  value,
  onChange,
}: {
  items: readonly { id: T; label: string }[]
  value: T
  onChange: (id: T) => void
}) {
  return (
    <div className="ui-tabs" role="tablist">
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          role="tab"
          aria-selected={item.id === value}
          className={`ui-tabs__tab${item.id === value ? ' ui-tabs__tab--active' : ''}`}
          onClick={() => onChange(item.id)}
        >
          {item.label}
        </button>
      ))}
    </div>
  )
}
