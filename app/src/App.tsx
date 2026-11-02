import { HashRouter, Navigate, Route, Routes } from 'react-router-dom'
import { SpaceBackground } from './components/background/SpaceBackground'
import './components/background/space.css'
import { AdminBanner } from './components/admin-banner/AdminBanner'
import { ErrorBoundary } from './components/ErrorBoundary'
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
          <AdminBanner />
          <Routes>
            {features.map((f) => (
              <Route
                key={f.id}
                path={`/${f.id}/*`}
                element={
                  // one broken screen doesn't take the shell down
                  <ErrorBoundary name={f.title}>
                    <f.component />
                  </ErrorBoundary>
                }
              />
            ))}
            <Route path="*" element={<Navigate to={defaultFeature ? `/${defaultFeature.id}` : '/'} replace />} />
          </Routes>
        </main>
      </div>
      <PatchNotesHost />
    </HashRouter>
  )
}
