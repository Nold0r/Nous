import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { LocateFixed, Maximize, Minus, Plus } from 'lucide-react'
import type { Position } from '@app/lib/game-data'
import { headingOnScreen, mapUrl, pxPerMeter, toPicture, type FloorMap, type MapLayer } from './maps'
import { MarkLegend, MarkPin } from './MapMarks'
import { isDone, useMarkFilter, useMarks, visibleMarks, type Progress } from './marks'

const MIN_ZOOM = 0.05
const MAX_ZOOM = 4
/** screen px per meter when following the player */
const FOLLOW_PX_PER_M = 4
/** a glide between two positions lasts as long as the gap between them, within these ms */
const MIN_GLIDE = 50
const MAX_GLIDE = 400
/** farther than this in meters is a teleport: jump */
const TELEPORT_M = 20

/** Where the arrow is on the picture and where it points. */
interface Pose {
  x: number
  y: number
  /** degrees clockwise from up, not wrapped */
  h: number
}

/**
 * The arrow glides from where it is to each new position over the time since
 * the previous one: steady speed, at most one update behind. Turns take the
 * short way round; teleports and floor changes jump.
 */
function useGlide(target: Pose | null, snapPx: number): Pose | null {
  const [shown, setShown] = useState<Pose | null>(null)
  const cur = useRef<Pose | null>(null)
  const last = useRef(0)

  useEffect(() => {
    if (!target) {
      cur.current = null
      return
    }
    const t0 = performance.now()
    const gap = t0 - last.current
    last.current = t0
    const from = cur.current
    const far = !from || Math.hypot(target.x - from.x, target.y - from.y) > snapPx
    const dur = far ? 0 : Math.min(MAX_GLIDE, Math.max(MIN_GLIDE, gap))
    const start = from ?? target
    const turn = ((((target.h - start.h) % 360) + 540) % 360) - 180
    let raf = 0
    const step = (t: number) => {
      const k = dur ? Math.min(1, (t - t0) / dur) : 1
      const p = { x: start.x + (target.x - start.x) * k, y: start.y + (target.y - start.y) * k, h: start.h + turn * k }
      cur.current = p
      setShown(p)
      if (k < 1) raf = requestAnimationFrame(step)
    }
    raf = requestAnimationFrame(step)
    return () => cancelAnimationFrame(raf)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target?.x, target?.y, target?.h, snapPx])

  return target ? (shown ?? target) : null
}

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
export function MapView({
  map,
  layer,
  player,
  progress,
}: {
  map: FloorMap
  layer: MapLayer
  player: Position | null
  /** what the game told about this floor's chests and puzzles */
  progress: Progress
}) {
  const box = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ w: 0, h: 0 })
  const [view, setView] = useState<View | null>(null)
  const [follow, setFollow] = useState(true)
  const [failed, setFailed] = useState<string | null>(null)
  // The previous floor's roads stay under the new ones until these fade in.
  // Load states reset while rendering, not in an effect: an effect ran after
  // cached pictures had loaded and left "loading" up for good. They are per
  // showing, so a picture seen before fades in again.
  const [roads, setRoads] = useState({ map, file: layer.file })
  const [under, setUnder] = useState<string | null>(null)
  const [ready, setReady] = useState<string | null>(null)
  const [bgReady, setBgReady] = useState<string | null>(null)
  if (roads.file !== layer.file || roads.map !== map) {
    setRoads({ map, file: layer.file })
    setUnder(roads.map === map && ready === roads.file ? roads.file : null)
    setReady(null)
    if (roads.map !== map) setBgReady(null)
  }
  const drag = useRef<{ x: number; y: number; vx: number; vy: number } | null>(null)
  const marks = useMarks(map)
  const [filter, setFilter] = useMarkFilter()
  const visible = visibleMarks(marks, map, layer, filter, progress)
  const byKey = new Map(visible.map((m) => [m.key, m]))

  const target = player ? { ...toPicture(map, player), h: headingOnScreen(map, player) } : null
  const at = useGlide(target, TELEPORT_M * pxPerMeter(map))
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

  // first view: on the player, else the whole map
  const hasPlayer = !!at
  useEffect(() => {
    if (!size.w || !size.h || view) return
    setView(follow && at ? centreOn(at, followZoom) : fitAll())
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [size, follow, hasPlayer, map, view])

  // following moves the map with the arrow, every frame of its glide
  const shown = view && follow && at ? centreOn(at, view.zoom) : view

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
    if (e.button !== 0 || !shown) return
    ;(e.target as Element).setPointerCapture(e.pointerId)
    drag.current = { x: e.clientX, y: e.clientY, vx: shown.x, vy: shown.y }
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
      {shown && (
        <div
          className="map-view__picture"
          style={{ width: map.width, height: map.height, transform: `translate(${shown.x}px, ${shown.y}px) scale(${shown.zoom})` }}
        >
          {/* the game's way: the background, then the roads of the floor */}
          {map.background && (
            <img
              key={map.background}
              src={mapUrl(map.background)}
              alt=""
              draggable={false}
              className={bgReady === map.background ? 'is-ready' : undefined}
              onLoad={() => setBgReady(map.background)}
            />
          )}
          {under && under !== layer.file && (
            <img key={under} src={mapUrl(under)} alt="" draggable={false} className="is-under" />
          )}
          <img
            key={layer.file}
            src={mapUrl(layer.file)}
            alt=""
            draggable={false}
            className={ready === layer.file ? 'is-ready' : undefined}
            onLoad={() => setReady(layer.file)}
            onError={() => {
              setFailed(layer.file)
              setUnder(null)
            }}
            onAnimationEnd={() => setUnder(null)}
          />
        </div>
      )}

      {/* a puzzle's start and its reward, when both are on this floor */}
      {shown && (
        <svg className="map-links" aria-hidden>
          {visible.map((m) => {
            const end = m.pair && !m.reward ? byKey.get(m.pair) : undefined
            if (!end) return null
            const sx = (p: { x: number }) => shown.x + p.x * shown.zoom
            const sy = (p: { y: number }) => shown.y + p.y * shown.zoom
            return (
              <line
                key={m.key}
                x1={sx(m)}
                y1={sy(m)}
                x2={sx(end)}
                y2={sy(end)}
                className={isDone(m, progress) ? 'is-opened' : undefined}
              />
            )
          })}
        </svg>
      )}

      {shown &&
        visible.map((m) => (
          <MarkPin
            key={m.key}
            mark={m}
            left={shown.x + m.x * shown.zoom}
            top={shown.y + m.y * shown.zoom}
            opened={!!isDone(m, progress)}
          />
        ))}

      {shown && at && (
        <div
          className="map-player"
          style={{
            left: shown.x + at.x * shown.zoom,
            top: shown.y + at.y * shown.zoom,
            transform: `translate(-50%, -50%) rotate(${at.h}deg)`,
          }}
          aria-label="Вы здесь"
        >
          <svg viewBox="0 0 24 24" width="26" height="26">
            <path d="M12 2 L20 21 L12 16.5 L4 21 Z" />
          </svg>
        </div>
      )}

      {ready !== layer.file && !under && (
        <div className="map-view__status">
          {failed === layer.file ? 'Не удалось загрузить карту' : 'Загрузка карты…'}
        </div>
      )}

      <MarkLegend marks={marks} filter={filter} onChange={setFilter} progress={progress} />

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
