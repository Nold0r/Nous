import { useEffect, useRef, useState, type CSSProperties } from 'react'
import { artCenters, spineFiles, type ArtCenter } from '../../lib/spine/manifest'
import { playSpine, type SpinePlayer } from '../../lib/spine/engine'
import './spine.css'

const CANVAS = 2048
/** HSR Optimizer draws the 2048 px art 950 px wide in a 422 px frame (×z), centred on the art's centre point. */
const SCALE = 950 / 422

/** Places the art so `c` (in 2048 px art coordinates) is in the middle of the frame. */
const place = (c: ArtCenter) =>
  ({ '--art-w': SCALE * c.z, '--art-x': c.x / CANVAS, '--art-y': c.y / CANVAS }) as CSSProperties

/**
 * A character's animated splash art (the game's own Spine animation), or the
 * static `fallback` picture while it loads and when there is none, framed
 * like HSR Optimizer frames it (see lib/spine/manifest). Fills its parent and
 * crops. The animation pauses while the window is hidden or out of focus.
 */
export function SpineArt({ characterId, fallback }: { characterId: number; fallback: string }) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const player = useRef<SpinePlayer | null>(null)
  const [ready, setReady] = useState(false)
  const files = spineFiles(characterId)
  const centers = artCenters(characterId)

  useEffect(() => {
    if (!files || !canvas.current) return
    setReady(false)
    const abort = new AbortController()
    let gone = false
    playSpine(canvas.current, files.baseUrl, files.files, abort.signal, () => !gone && setReady(false))
      .then((p) => {
        if (gone) return p.dispose()
        player.current = p
        if (document.hidden || !document.hasFocus()) p.pause()
        setReady(true)
      })
      .catch((e) => {
        if ((e as DOMException)?.name !== 'AbortError') console.warn('[spine] no animated art for', characterId, e)
      })

    const sync = () => (document.hidden || !document.hasFocus() ? player.current?.pause() : player.current?.resume())
    document.addEventListener('visibilitychange', sync)
    window.addEventListener('focus', sync)
    window.addEventListener('blur', sync)
    return () => {
      gone = true
      abort.abort()
      player.current?.dispose()
      player.current = null
      document.removeEventListener('visibilitychange', sync)
      window.removeEventListener('focus', sync)
      window.removeEventListener('blur', sync)
    }
    // files is derived from characterId
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [characterId])

  return (
    <span className={`spine-art${ready ? ' spine-art--ready' : ''}`}>
      <img className="spine-art__still" style={place(centers.image)} src={fallback} alt="" draggable={false} />
      {files && (
        <canvas
          key={characterId}
          ref={canvas}
          className="spine-art__canvas"
          style={place(centers.spine)}
          width={CANVAS}
          height={CANVAS}
        />
      )}
    </span>
  )
}
