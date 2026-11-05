import {
  AnimationState,
  AnimationStateData,
  AssetManager,
  AtlasAttachmentLoader,
  ManagedWebGLRenderingContext,
  SceneRenderer,
  Skeleton,
  SkeletonBinary,
  Vector2,
} from '@esotericsoftware/spine-webgl'

// Plays a character's animated art (Spine skeletons) on a square canvas.
// Adapted from HSR Optimizer's spineEngine.ts (github.com/fribbels/hsr-optimizer,
// MIT). The Spine runtime itself is under the Spine Runtimes License, see
// docs/Архитектура/Модуль персонажей.md.

export interface SpinePlayer {
  pause(): void
  resume(): void
  dispose(): void
}

const GL_ONE = 1
const GL_ONE_MINUS_SRC_COLOR = 0x0301
const GL_ONE_MINUS_SRC_ALPHA = 0x0303

/**
 * The arts use Multiply and Screen slots; spine-webgl's blending assumes an
 * opaque canvas. Keep alpha "over" so shading never punches holes in a
 * transparent canvas, and use true Screen for Screen slots.
 */
function patchBlendModes(gl: WebGLRenderingContext) {
  const orig = gl.blendFuncSeparate.bind(gl)
  gl.blendFuncSeparate = (srcRGB: number, dstRGB: number, srcAlpha: number) => {
    if (srcAlpha === GL_ONE_MINUS_SRC_COLOR) orig(srcRGB, GL_ONE_MINUS_SRC_COLOR, GL_ONE, GL_ONE_MINUS_SRC_ALPHA)
    else orig(srcRGB, dstRGB, GL_ONE, GL_ONE_MINUS_SRC_ALPHA)
  }
}

/** Loads the skeletons, fits them to the canvas and starts the idle loop. Rejects if WebGL or the files fail. */
export async function playSpine(
  canvas: HTMLCanvasElement,
  baseUrl: string,
  files: { skel: string; atlas: string }[],
  signal: AbortSignal,
  onError: () => void,
): Promise<SpinePlayer> {
  const options = { alpha: true, premultipliedAlpha: true, antialias: false, preserveDrawingBuffer: false }
  const gl = (canvas.getContext('webgl2', options) || canvas.getContext('webgl', options)) as WebGLRenderingContext | null
  if (!gl) throw new Error('WebGL недоступен')
  patchBlendModes(gl)

  const context = new ManagedWebGLRenderingContext(gl)
  const assets = new AssetManager(context, baseUrl)
  for (const f of files) {
    assets.loadBinary(f.skel)
    assets.loadTextureAtlas(f.atlas)
  }
  try {
    await new Promise<void>((resolve, reject) => {
      const check = () => {
        if (signal.aborted) return reject(new DOMException('aborted', 'AbortError'))
        if (!assets.isLoadingComplete()) return requestAnimationFrame(check)
        if (assets.hasErrors()) reject(new Error(JSON.stringify(assets.getErrors())))
        else resolve()
      }
      check()
    })
  } catch (e) {
    assets.dispose()
    throw e
  }

  const read = (f: { skel: string; atlas: string }, scale: number) => {
    const binary = new SkeletonBinary(new AtlasAttachmentLoader(assets.require(f.atlas)))
    binary.scale = scale
    return binary.readSkeletonData(assets.require(f.skel))
  }

  // bounds of the setup pose at scale 1, to fit the art into the square
  let minX = Infinity
  let minY = Infinity
  let maxX = -Infinity
  let maxY = -Infinity
  for (const f of files) {
    const skeleton = new Skeleton(read(f, 1))
    skeleton.setToSetupPose()
    skeleton.updateWorldTransform()
    const offset = new Vector2()
    const size = new Vector2()
    skeleton.getBounds(offset, size, [])
    minX = Math.min(minX, offset.x)
    minY = Math.min(minY, offset.y)
    maxX = Math.max(maxX, offset.x + size.x)
    maxY = Math.max(maxY, offset.y + size.y)
  }
  const side = canvas.width
  const fit = side / Math.max(maxX - minX, maxY - minY)

  const entries = files.map((f) => {
    const data = read(f, fit)
    const skeleton = new Skeleton(data)
    skeleton.setToSetupPose()
    const state = new AnimationState(new AnimationStateData(data))
    const idle = data.animations.find((a) => a.name.toLowerCase().includes('idle')) ?? data.animations[0]
    if (idle) state.setAnimation(0, idle.name, true)
    skeleton.updateWorldTransform()
    return { skeleton, state }
  })

  const renderer = new SceneRenderer(canvas, context)
  renderer.camera.position.x = Math.round((minX + (maxX - minX) / 2) * fit)
  renderer.camera.position.y = Math.round((minY + (maxY - minY) / 2) * fit)
  renderer.camera.zoom = 1
  renderer.camera.setViewport(side, side)
  renderer.camera.update()

  let raf: number | null = null
  let last = performance.now()
  let disposed = false

  const dispose = () => {
    if (disposed) return
    disposed = true
    if (raf != null) cancelAnimationFrame(raf)
    raf = null
    canvas.removeEventListener('webglcontextlost', lost)
    renderer.dispose()
    assets.dispose()
  }
  const lost = () => {
    dispose()
    onError()
  }
  canvas.addEventListener('webglcontextlost', lost)

  const frame = (now: number) => {
    if (disposed) return
    raf = requestAnimationFrame(frame)
    try {
      const delta = Math.min((now - last) / 1000, 0.1)
      last = now
      for (const e of entries) {
        e.state.update(delta)
        e.state.apply(e.skeleton)
        e.skeleton.updateWorldTransform()
      }
      gl.viewport(0, 0, side, side)
      gl.clearColor(0, 0, 0, 0)
      gl.clear(gl.COLOR_BUFFER_BIT)
      renderer.begin()
      for (const e of entries) renderer.drawSkeleton(e.skeleton, false)
      renderer.end()
    } catch (err) {
      console.error('[spine] render error', err)
      dispose()
      onError()
    }
  }

  if (signal.aborted) {
    dispose()
    throw new DOMException('aborted', 'AbortError')
  }
  raf = requestAnimationFrame(frame)

  return {
    pause() {
      if (disposed || raf == null) return
      cancelAnimationFrame(raf)
      raf = null
    },
    resume() {
      if (disposed || raf != null) return
      last = performance.now()
      raf = requestAnimationFrame(frame)
    },
    dispose,
  }
}
