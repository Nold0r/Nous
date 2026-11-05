/**
 * One skill: round icon, kind ("Навык"), name, level. `bonus` is added on
 * top (eidolons) and shown in green, the cap grows with it.
 */
export function SkillRow({
  icon,
  kind,
  name,
  level,
  max,
  bonus = 0,
}: {
  icon: string
  kind: string
  name: string
  level: number
  max: number
  bonus?: number
}) {
  return (
    <div className="ui-skill" title={name}>
      <img src={icon} alt="" draggable={false} />
      <span className="ui-skill__text">
        <span className="ui-skill__kind">{kind}</span>
        <span className="ui-skill__name">{name}</span>
      </span>
      <span className="ui-skill__level">
        {level + bonus}
        {bonus > 0 && (
          <span className="ui-skill__bonus" title="от эйдолонов">
            +{bonus}
          </span>
        )}
        <span className="ui-skill__max">/{max + bonus}</span>
      </span>
    </div>
  )
}
