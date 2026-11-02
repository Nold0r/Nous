import { NavLink } from 'react-router-dom'
import logo from '../../assets/brand/nous-logo.svg'
import { groupFeatures, type FeatureGroup } from '../../features/registry'
import { useModules } from '../../features/module-context'
import { GameIcon } from '../../ui'
import { PlayButton } from './PlayButton'
import { ProfileCard } from './ProfileCard'
import { UpdateNotice } from './UpdateNotice'

function NavGroup({ group }: { group: FeatureGroup }) {
  return (
    <div className="nav-group">
      {group.category.title && <div className="nav-group__title">{group.category.title}</div>}
      {group.features.map((f) => (
        <NavLink key={f.id} to={`/${f.id}`} className={({ isActive }) => `nav-item${isActive ? ' nav-item--active' : ''}`}>
          <GameIcon icon={f.icon} size={22} className="nav-item__icon" />
          <span className="nav-item__title">{f.title}</span>
          {f.badge && <span className="nav-item__badge">{f.badge}</span>}
        </NavLink>
      ))}
    </div>
  )
}

export function Sidebar() {
  const { screens } = useModules()
  const top = groupFeatures(screens, false)
  const bottom = groupFeatures(screens, true)

  return (
    <aside className="sidebar acrylic">
      <div className="sidebar__brand">
        <img className="sidebar__logo" src={logo} alt="" draggable={false} />
        <span className="sidebar__title">Nous</span>
      </div>

      <ProfileCard />
      <PlayButton />

      <nav className="sidebar__nav">
        {top.map((g) => (
          <NavGroup key={g.category.id} group={g} />
        ))}
      </nav>

      {bottom.length > 0 && (
        <nav className="sidebar__bottom">
          {bottom.map((g) => (
            <NavGroup key={g.category.id} group={g} />
          ))}
        </nav>
      )}

      <UpdateNotice />
    </aside>
  )
}
