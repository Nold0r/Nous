/**
 * A row of round steps that are either reached or not, like eidolons
 * E1–E6: picture, short label, full text in the tooltip.
 */
export function RankRow({ items }: { items: { image: string; label: string; active: boolean; title?: string }[] }) {
  return (
    <div className="ui-ranks" style={{ gridTemplateColumns: `repeat(${items.length}, 1fr)` }}>
      {items.map((it) => (
        <span key={it.label} className={`ui-ranks__item${it.active ? ' ui-ranks__item--on' : ''}`} title={it.title}>
          <img src={it.image} alt="" draggable={false} />
          <span>{it.label}</span>
        </span>
      ))}
    </div>
  )
}
