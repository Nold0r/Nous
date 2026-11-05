import { StatIcon } from './StatIcon'

/**
 * A stat as an icon and a value, e.g. a relic substat. `rolls` draws one ◆
 * per upgrade; `label` is the tooltip, shown as text only with `showLabel`.
 * `main` is the highlighted band of a main stat.
 */
export function StatLine({
  icon,
  label,
  value,
  rolls = 0,
  main,
  showLabel,
}: {
  icon: string
  label: string
  value: string
  /** upgrades on top of the starting roll */
  rolls?: number
  main?: boolean
  showLabel?: boolean
}) {
  return (
    <div className={`ui-statline${main ? ' ui-statline--main' : ''}`} title={label}>
      <StatIcon icon={icon} size={main ? 20 : 16} title={label} />
      {showLabel && <span className="ui-statline__label">{label}</span>}
      {rolls > 0 && (
        <span className="ui-statline__rolls" aria-label={`улучшений: ${rolls}`}>
          {Array.from({ length: rolls }, (_, i) => (
            <i key={i} />
          ))}
        </span>
      )}
      <span className="ui-statline__value">{value}</span>
    </div>
  )
}
