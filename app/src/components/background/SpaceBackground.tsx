import { useEffect, useRef } from 'react'

/**
 * Animated backdrop modelled on the HSR character screen: nebula, stars, a
 * warm glow low in the sky and fog rising from the bottom. Layers move with the mouse at different depths (parallax) via the
 * --px / --py CSS variables, see .space in index.css.
 */
export function SpaceBackground() {
  const rootRef = useRef<HTMLDivElement>(null)
  const starsRef = useRef<HTMLCanvasElement>(null)

  // parallax: ease the layers towards the pointer position
  useEffect(() => {
    const root = rootRef.current
    if (!root) return
    const target = { x: 0, y: 0 }
    const current = { x: 0, y: 0 }
    let frame = 0

    const onMove = (e: PointerEvent) => {
      target.x = e.clientX / window.innerWidth - 0.5
      target.y = e.clientY / window.innerHeight - 0.5
    }
    const tick = () => {
      current.x += (target.x - current.x) * 0.04
      current.y += (target.y - current.y) * 0.04
      root.style.setProperty('--px', current.x.toFixed(4))
      root.style.setProperty('--py', current.y.toFixed(4))
      frame = requestAnimationFrame(tick)
    }

    window.addEventListener('pointermove', onMove)
    frame = requestAnimationFrame(tick)
    return () => {
      window.removeEventListener('pointermove', onMove)
      cancelAnimationFrame(frame)
    }
  }, [])

  // star field: every star is a blurred sprite and slowly breathes at its own pace
  useEffect(() => {
    const canvas = starsRef.current
    const ctx = canvas?.getContext('2d')
    if (!canvas || !ctx) return

    const sprite = makeStarSprite()

    type Star = {
      x: number
      y: number
      /** drawn sprite size in css px */
      size: number
      /** base brightness */
      a: number
      /** breathing: angular speed, phase and depth (0 = steady) */
      speed: number
      phase: number
      depth: number
    }
    let stars: Star[] = []
    let frame = 0

    const resize = () => {
      const dpr = window.devicePixelRatio || 1
      canvas.width = canvas.clientWidth * dpr
      canvas.height = canvas.clientHeight * dpr
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
      const count = Math.round((canvas.clientWidth * canvas.clientHeight) / 2600)
      stars = Array.from({ length: count }, () => {
        const bright = Math.random() < 0.07
        return {
          x: Math.random() * canvas.clientWidth,
          // denser towards the top, the bottom is covered by haze and fog
          y: Math.pow(Math.random(), 1.3) * canvas.clientHeight * 0.85,
          size: bright ? 16 + Math.random() * 16 : 4 + Math.random() * 5,
          a: bright ? 0.7 + Math.random() * 0.3 : 0.25 + Math.random() * 0.6,
          // one slow breath every ~5-12 s
          speed: (Math.PI * 2) / (5 + Math.random() * 7),
          phase: Math.random() * Math.PI * 2,
          depth: bright ? 0.2 + Math.random() * 0.15 : Math.random() < 0.6 ? 0.1 + Math.random() * 0.2 : 0,
        }
      })
    }

    const draw = (time: number) => {
      const t = time / 1000
      ctx.clearRect(0, 0, canvas.clientWidth, canvas.clientHeight)
      ctx.globalCompositeOperation = 'lighter'
      for (const s of stars) {
        // two sines with different rates make the flicker less regular
        // smooth breathing: a single slow sine, brightness and size swell together
        const wave = 0.5 + 0.5 * Math.sin(t * s.speed + s.phase)
        const k = 1 - s.depth * (1 - wave)
        const size = s.size * (0.94 + 0.06 * k)
        ctx.globalAlpha = s.a * k
        ctx.drawImage(sprite, s.x - size / 2, s.y - size / 2, size, size)
      }
      ctx.globalAlpha = 1
      ctx.globalCompositeOperation = 'source-over'
      frame = requestAnimationFrame(draw)
    }

    resize()
    window.addEventListener('resize', resize)
    frame = requestAnimationFrame(draw)
    return () => {
      window.removeEventListener('resize', resize)
      cancelAnimationFrame(frame)
    }
  }, [])

  return (
    <div className="space" ref={rootRef} aria-hidden>
      <div className="space__layer space__nebula" />
      <canvas className="space__layer space__stars" ref={starsRef} />
      <div className="space__layer space__haze" />
      <div className="space__fog space__fog--back" />
      <div className="space__fog space__fog--front" />
      <div className="space__vignette" />
    </div>
  )
}

/**
 * Soft star sprite, rendered once: a radial gradient whose stops follow a
 * gaussian falloff (tight core + wide glow), i.e. a blurred point of light
 * without any hard edge. Built by hand so it doesn't depend on canvas filters.
 */
function makeStarSprite(): HTMLCanvasElement {
  const size = 64
  const c = document.createElement('canvas')
  c.width = c.height = size
  const g = c.getContext('2d')!
  const mid = size / 2

  const gradient = g.createRadialGradient(mid, mid, 0, mid, mid, mid)
  const steps = 24
  for (let i = 0; i <= steps; i++) {
    const d = i / steps // 0 = centre, 1 = sprite edge
    const core = Math.exp(-((d / 0.07) ** 2))
    const glow = 0.45 * Math.exp(-((d / 0.3) ** 2))
    const a = Math.min(1, core + glow) * (1 - d) // reaches 0 at the edge
    gradient.addColorStop(d, `rgba(255, ${Math.round(236 + 19 * core)}, ${Math.round(240 + 15 * core)}, ${a.toFixed(3)})`)
  }
  g.fillStyle = gradient
  g.fillRect(0, 0, size, size)
  return c
}
