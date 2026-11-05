import { Lock, Trash2 } from 'lucide-react'
import { StatLine } from '../../ui'
import { characterIcon, formatStat, type RelicView, type Stat } from '../../lib/game-tables'
import './relic.css'

// Pieces of a relic shared by the inventory and characters screens.

/** Main stat band and substats; `labels` writes stat names, otherwise icons only in two columns. */
export function RelicStats({ relic, labels }: { relic: RelicView; labels?: boolean }) {
  const line = (s: Stat, main?: boolean) => (
    <StatLine
      key={s.property}
      icon={s.icon}
      label={s.name}
      value={formatStat(s)}
      rolls={s.rolls ? s.rolls - 1 : 0}
      main={main}
      showLabel={labels || main}
    />
  )
  return (
    <>
      {relic.main && line(relic.main, true)}
      <div className={labels ? 'relic-subs' : 'relic-subs relic-subs--grid'}>{relic.subs.map((s) => line(s))}</div>
    </>
  )
}

/** Lock and "to salvage" marks, or nothing. */
export function RelicBadges({ relic }: { relic: RelicView }) {
  if (!relic.relic.locked && !relic.relic.discarded) return null
  return (
    <>
      {relic.relic.locked && <Lock size={14} aria-label="Заблокирована" />}
      {relic.relic.discarded && <Trash2 size={14} className="relic-discarded" aria-label="На разбор" />}
    </>
  )
}

/** Who wears it: round picture and name, or "Не надета". */
export function RelicWearer({ relic, name }: { relic: RelicView; name: string | null }) {
  if (relic.relic.equippedBy == null || !name) return <span className="text-dim">Не надета</span>
  return (
    <span className="relic-wearer">
      <img src={characterIcon(relic.relic.equippedBy)} alt="" loading="lazy" />
      {name}
    </span>
  )
}
