import { HashRouter, Navigate, Route, Routes } from 'react-router-dom'
import { SpaceBackground } from './components/background/SpaceBackground'
import './components/background/space.css'
import { PatchNotesHost } from './components/patch-notes/PatchNotes'
import { Sidebar } from './components/sidebar/Sidebar'
import { defaultFeature, features } from './features/registry'

export default function App() {
  return (
    <HashRouter>
      <SpaceBackground />
      <div className="layout">
        <Sidebar />
        <main className="content">
          <Routes>
            {features.map((f) => (
              <Route key={f.id} path={`/${f.id}/*`} element={<f.component />} />
            ))}
            <Route path="*" element={<Navigate to={defaultFeature ? `/${defaultFeature.id}` : '/'} replace />} />
          </Routes>
        </main>
      </div>
      <PatchNotesHost />
    </HashRouter>
  )
}
