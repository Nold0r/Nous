import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './ui/tokens.css'
import './ui/base.css'
import './ui/components.css'
import './shell.css'
import App from './App.tsx'
import { overlayModule } from './features/registry'

// `#/overlay/<module>/...` is a module's own window over the game (e.g. Veritas
// widgets): no shell, transparent page.
const overlay = overlayModule(window.location.hash)
if (overlay) document.documentElement.classList.add('overlay-root')
const Overlay = overlay?.overlay
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {Overlay ? <Overlay /> : <App />}
  </StrictMode>,
)
