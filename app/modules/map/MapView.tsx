import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { LocateFixed, Maximize, Minus, Plus } from 'lucide-react'
import type { Position } from '@app/lib/game-data'
import { headingOnScreen, mapUrl, pxPerMeter, toPicture, type FloorMap, type MapLayer } from './maps'

const MIN_ZOOM = 0.05
const MAX_ZOOM = 4
/** screen px per meter when following the player */
const FOLLOW_PX_PER_M = 4

interface View {
  zoom: number
  /** screen px of the picture's top-left corner */
  x: number
  y: number
}

/**
 * The floor picture, dragged and zoomed like a map, with the player's arrow.
 * Following the player keeps them centred until the map is dragged.
 */
export function MapView({ map, layer, player }: { map: FloorMap; layer: MapLayer; player: Position | null }) {
  const box = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ w: 0, h: 0 })
  const [view, setView] = useState<View | null>(null)
  const [follow, setFollow] = useState(true)
  const [state, setState] = useState<'loading' | 'ready' | 'error'>('loading')
  const drag = useRef<{ x: number; y: number; vx: number; vy: number } | null>(null)

  const at = player ? toPicture(map, player) : null
  const followZoom = FOLLOW_PX_PER_M / pxPerMeter(map)

  useLayoutEffect(() => {
    const el = box.current
    if (!el) return
    const ro = new ResizeObserver(([e]) => setSize({ w: e.contentRect.width, h: e.contentRect.height }))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  const fitAll = useCallback((): View => {
    const zoom = Math.min(size.w / map.width, size.h / map.height)
    return { zoom, x: (size.w - map.width * zoom) / 2, y: (size.h - map.height * zoom) / 2 }
  }, [size, map])

  const centreOn = useCallback(
    (p: { x: number; y: number }, zoom: number): View => ({ zoom, x: size.w / 2 - p.x * zoom, y: size.h / 2 - p.y * zoom }),
    [size],
  )

  // a new location: start over
  useEffect(() => {
    setView(null)
    setFollow(true)
  }, [map])

  useEffect(() => setState('loading'), [layer.file])

  // first view, and following the player
  useEffect(() => {
    if (!size.w || !size.h) return
    if (follow && at) setView((v) => centreOn(at, v?.zoom ?? followZoom))
    else if (!view) setView(fitAll())
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size, follow, at?.x, at?.y, map])

  const zoomAround = (factor: number, cx = size.w / 2, cy = size.h / 2) =>
    setView((v) => {
      if (!v) return v
      const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.zoom * factor))
      const f = zoom / v.zoom
      // following: zoom around the player, who stays centred
      if (follow && at) return centreOn(at, zoom)
      return { zoom, x: cx - (cx - v.x) * f, y: cy - (cy - v.y) * f }
    })

  const onWheel = (e: React.WheelEvent) => {
    const r = box.current!.getBoundingClientRect()
    zoomAround(Math.exp(-e.deltaY * 0.0015), e.clientX - r.left, e.clientY - r.top)
  }

  const onPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0 || !view) return
    ;(e.target as Element).setPointerCapture(e.pointerId)
    drag.current = { x: e.clientX, y: e.clientY, vx: view.x, vy: view.y }
  }
  const onPointerMove = (e: React.PointerEvent) => {
    const d = drag.current
    if (!d) return
    const dx = e.clientX - d.x
    const dy = e.clientY - d.y
    if (follow && Math.hypot(dx, dy) < 4) return
    setFollow(false)
    setView((v) => v && { ...v, x: d.vx + dx, y: d.vy + dy })
  }
  const onPointerUp = () => {
    drag.current = null
  }

  return (
    <div
      ref={box}
      className="map-view"
      onWheel={onWheel}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onDoubleClick={() => {
        setFollow(false)
        setView(fitAll())
      }}
    >
      {view && (
        <div
          className="map-view__picture"
          style={{ width: map.width, height: map.height, transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}
        >
          {/* the game's way: the background, then the roads of the floor */}
          {map.background && <img src={mapUrl(map.background)} alt="" draggable={false} />}
          <img
            key={layer.file}
            src={mapUrl(layer.file)}
            alt=""
            draggable={false}
            onLoad={() => setState('ready')}
            onError={() => setState('error')}
          />
        </div>
      )}

      {view && at && player && (
        <div
          className="map-player"
          style={{
            left: view.x + at.x * view.zoom,
            top: view.y + at.y * view.zoom,
            transform: `translate(-50%, -50%) rotate(${headingOnScreen(map, player)}deg)`,
          }}
          aria-label="Вы здесь"
        >
          <svg viewBox="0 0 24 24" width="26" height="26">
            <path d="M12 2 L20 21 L12 16.5 L4 21 Z" />
          </svg>
        </div>
      )}

      {state !== 'ready' && (
        <div className="map-view__status">
          {state === 'loading' ? 'Загрузка карты…' : 'Не удалось загрузить карту'}
        </div>
      )}

      <div className="map-controls" onPointerDown={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
        <button type="button" title="Приблизить" onClick={() => zoomAround(1.4)}>
          <Plus size={16} />
        </button>
        <button type="button" title="Отдалить" onClick={() => zoomAround(1 / 1.4)}>
          <Minus size={16} />
        </button>
        <button
          type="button"
          title="Вся карта"
          onClick={() => {
            setFollow(false)
            setView(fitAll())
          }}
        >
          <Maximize size={16} />
        </button>
        <button
          type="button"
          title="Следить за игроком"
          className={follow ? 'map-controls__on' : undefined}
          disabled={!at}
          onClick={() => {
            setFollow(true)
            if (at) setView((v) => centreOn(at, Math.max(v?.zoom ?? followZoom, followZoom / 2)))
          }}
        >
          <LocateFixed size={16} />
        </button>
      </div>
    </div>
  )
}
