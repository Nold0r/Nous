import { CircleCheck, Circle } from 'lucide-react'

export interface SetEffect {
  name: string
  icon?: string
  /** pieces worn; without it the bonuses are listed with no checks */
  count?: number
  bonuses: { pieces: number; text: string }[]
}

/**
 * Set bonuses like the game's "Эффект набора": set name in gold, each
 * bonus with a check when enough pieces are worn, dimmed otherwise
 * (no checks when `count` is unknown, e.g. a relic on its own).
 */
export function SetEffects({ sets }: { sets: SetEffect[] }) {
  return (
    <div className="ui-sets">
      {sets.map((s) => (
        <div key={s.name} className="ui-sets__set">
          <div className="ui-sets__name">
            {s.icon && <img src={s.icon} alt="" />}
            {s.name}
            {s.count != null && <span className="ui-sets__count">×{s.count}</span>}
          </div>
          {s.bonuses.map((b) => {
            const known = s.count != null
            const on = known && s.count! >= b.pieces
            return (
              <div
                key={b.pieces}
                className={`ui-sets__bonus${on ? ' ui-sets__bonus--on' : ''}${known ? '' : ' ui-sets__bonus--plain'}`}
              >
                {known && (on ? <CircleCheck size={16} /> : <Circle size={16} />)}
                <p>
                  <b>{b.pieces} шт.:</b> {b.text}
                </p>
              </div>
            )
          })}
        </div>
      ))}
    </div>
  )
}
