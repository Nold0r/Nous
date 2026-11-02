import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './ui/tokens.css'
import './ui/base.css'
import './ui/components.css'
import './shell.css'
import App from './App.tsx'
import { OverlayApp } from './veritas/OverlayApp'
import './veritas/veritas.css'

const overlay = window.location.hash.startsWith('#/veritas-overlay')
const replayTarget = window.location.hash.startsWith('#/veritas-replay-target')
if (overlay) document.documentElement.classList.add('veritas-overlay-root')
createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {overlay ? <OverlayApp /> : replayTarget ? <main style={{padding:40,height:"100%",background:"#171219"}}><h1>Тестовая сцена Nous</h1><p>Сервер Veritas изображает бой. Игра не запущена, плагин не установлен.</p></main> : <App />}
  </StrictMode>,
)
