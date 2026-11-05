export interface OrbitSlot {
  image: string
  rarity: number
  /** e.g. "+15" */
  level: string
  /** slot name, for the tooltip and empty state */
  label: string
}

const SLOTS = ['head', 'hands', 'body', 'feet', 'sphere', 'rope'] as const

/**
 * A character's six relics placed like the game's relic screen: the four
 * cavern relics on a ring (head top left, feet top right, hands bottom
 * left, body bottom right), the two planar ornaments inside it. Pass slots
 * in the order head, hands, body, feet, sphere, rope; null is an empty slot.
 * Only pictures and levels: `onSelect` opens the details.
 */
export function RelicOrbit({
  slots,
  emptyLabels,
  onSelect,
}: {
  slots: (OrbitSlot | null)[]
  /** names of the six slots, shown when empty */
  emptyLabels: string[]
  onSelect?: (index: number) => void
}) {
  return (
    <div className="ui-orbit">
      <span className="ui-orbit__ring" aria-hidden />
      <span className="ui-orbit__ring ui-orbit__ring--inner" aria-hidden />
      {SLOTS.map((slot, i) => {
        const s = slots[i]
        const planar = i >= 4
        const cls = `ui-orbit__slot ui-orbit__slot--${slot}${planar ? ' ui-orbit__slot--planar' : ''}`
        if (!s) {
          return (
            <span key={slot} className={`${cls} ui-orbit__slot--empty`} title={`${emptyLabels[i]}: пусто`}>
              <span className="ui-orbit__disc">{emptyLabels[i]}</span>
            </span>
          )
        }
        const r = Math.min(5, Math.max(1, s.rarity))
        return (
          <button
            key={slot}
            type="button"
            className={`${cls} ui-orbit__slot--r${r}`}
            title={s.label}
            aria-label={s.label}
            onClick={() => onSelect?.(i)}
          >
            <span className="ui-orbit__disc">
              <img src={s.image} alt="" draggable={false} />
            </span>
            <span className="ui-orbit__level">{s.level}</span>
          </button>
        )
      })}
    </div>
  )
}
