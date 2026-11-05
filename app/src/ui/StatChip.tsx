import { StatIcon } from './StatIcon'

/** A stat bonus as a pill: icon, name, "+28%". For lists of small bonuses (traces). */
export function StatChip({ icon, label, value }: { icon: string; label: string; value: string }) {
  return (
    <span className="ui-stat-chip" title={label}>
      <StatIcon icon={icon} size={16} />
      {label}
      <b>{value}</b>
    </span>
  )
}
