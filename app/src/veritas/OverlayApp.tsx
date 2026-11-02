import { useState } from 'react'
import type { CSSProperties } from 'react'
import { EyeOff, Settings2 } from 'lucide-react'
import { Button } from '../ui'
import { BattleView } from './BattleView'
import { overlayAction, useOverlay } from './api'
import { themeVariables } from './presentation'
import './veritas.css'

/** Isolated route: no launcher shell, account loading, or focus request. */
export function OverlayApp() {
  const { state, patch, error } = useOverlay()
  const [actionError, setActionError] = useState<string | null>(null)
  if (!state) return null
  const { snapshot, settings } = state
  return <main className={`vt-overlay${settings.theme === 'light' ? ' vt-battle--light' : ''}`} style={{ ...themeVariables(settings), '--vt-scale': settings.scale, zoom: settings.scale } as CSSProperties} aria-label="Статистика боя Nous">
    <header className="vt-overlay__header"><div><h1>Nous <span>· Бой</span></h1><p>{state.replay ? 'Тестовое воспроизведение' : snapshot.active ? 'Бой идёт' : snapshot.connected ? 'Последний результат' : 'Соединение потеряно'}</p></div>
      {!settings.clickThrough && <div className="vt-actions"><Button size="sm" variant="ghost" icon={Settings2} title="Открыть настройки Nous" aria-label="Открыть настройки Nous" onClick={() => void overlayAction('settings').catch((e) => setActionError(String(e)))} /><Button size="sm" variant="ghost" icon={EyeOff} title="Скрыть оверлей" aria-label="Скрыть оверлей" onClick={() => void overlayAction('hide').catch((e) => setActionError(String(e)))} /></div>}
    </header>
    {(error || actionError) && <p className="vt-error" role="alert">{error || actionError}</p>}
    <BattleView battle={snapshot.battle} snapshot={snapshot} settings={settings} patch={patch} />
  </main>
}
