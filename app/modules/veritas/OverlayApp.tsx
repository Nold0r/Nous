import { useLayoutEffect, useRef } from 'react'
import type { CSSProperties } from 'react'
import { BattleView, Widget } from './BattleView'
import { useOverlay } from './api'
import { themeVariables, viewLabels } from './presentation'
import type { ViewKey } from './types'
import './veritas.css'

declare global {
  interface Window {
    /** Set by the composition host: reports the widget's content size. */
    nousOverlay?: { size: (width: number, height: number) => void }
  }
}

/** Stand-in for the game window in the isolated debug replay. */
function ReplayTarget() {
  return <main style={{ padding: 40, height: '100%', background: '#171219' }}><h1>Тестовая сцена Nous</h1><p>Сервер Veritas изображает бой. Игра не запущена, плагин не установлен.</p></main>
}

/**
 * The module's overlay page. `#/overlay/veritas/<widget>` is one widget in its
 * own popup (sized to its content, scaled by the host); without a widget it
 * is the single-window fallback showing every view.
 */
export function OverlayApp() {
  return window.location.hash.split('/')[3] === 'replay-target' ? <ReplayTarget /> : <Widgets />
}

function Widgets() {
  const { state, patch, error } = useOverlay()
  const widget = window.location.hash.split('/')[3] as ViewKey | undefined
  const root = useRef<HTMLElement>(null)
  const loaded = state != null

  // the host sizes the popup to the content; report every change
  useLayoutEffect(() => {
    const element = root.current
    const report = window.nousOverlay?.size
    if (!element || !report) return
    const observer = new ResizeObserver(() => report(element.offsetWidth, element.offsetHeight))
    observer.observe(element)
    return () => observer.disconnect()
  }, [loaded])

  if (!state) return null
  const { snapshot, settings } = state
  if (widget && widget in viewLabels) {
    return <main ref={root} className={`vt-widget${state.arranging ? ' vt-widget--arranging' : ''}`} style={themeVariables(settings) as CSSProperties} aria-label={viewLabels[widget]}>
      {state.arranging && <p className="vt-widget__hint">Перетащите</p>}
      <Widget name={widget} battle={snapshot.battle} snapshot={snapshot} settings={settings} patch={patch} />
    </main>
  }
  return <main className="vt-overlay" style={{ ...themeVariables(settings), '--vt-scale': settings.scale, zoom: settings.scale } as CSSProperties} aria-label="Статистика боя Nous">
    {error && <p className="vt-error" role="alert">{error}</p>}
    <BattleView battle={snapshot.battle} snapshot={snapshot} settings={settings} patch={patch} />
  </main>
}
