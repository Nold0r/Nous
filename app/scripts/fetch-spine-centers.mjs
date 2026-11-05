// Framing of the character arts, as tuned by hand in HSR Optimizer
// (github.com/fribbels/hsr-optimizer, MIT): for each character the point of
// the 2048 px art to put in the middle of the frame and the zoom, for the
// still portrait (imageCenter) and the animated one (spineCenter), plus the
// characters whose animation they turned off (disableSpine).
// Output: src/lib/spine/centers.json, read by src/lib/spine/manifest.ts.
// Run when the optimizer adds characters: npm run spine-centers
import { writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO = 'fribbels/hsr-optimizer'
const DIR = 'src/lib/conditionals/character/'
const root = path.dirname(fileURLToPath(import.meta.url))
const out = path.join(root, '..', 'src', 'lib', 'spine', 'centers.json')

const tree = await (await fetch(`https://api.github.com/repos/${REPO}/git/trees/main?recursive=1`)).json()
if (!tree.tree) throw new Error(`GitHub API: ${tree.message ?? 'no tree'}`)
const files = tree.tree.filter((f) => f.path.startsWith(DIR) && f.path.endsWith('.ts')).map((f) => f.path)

// "x: 938, y: 1088, z: 1.3" inside `name: { … }`
const center = (src, name) => {
  const m = src.match(new RegExp(`${name}\\s*:\\s*\\{([^}]*)\\}`))
  if (!m) return null
  const num = (k) => Number(m[1].match(new RegExp(`${k}\\s*:\\s*(-?[\\d.]+)`))?.[1])
  const c = { x: num('x'), y: num('y'), z: num('z') }
  return Object.values(c).every(Number.isFinite) ? c : null
}

const centers = {}
for (const file of files) {
  const src = await (await fetch(`https://raw.githubusercontent.com/${REPO}/main/${file}`)).text()
  // one file can export several ids with the same display (both Trailblazers)
  const ids = [...src.matchAll(/\bid\s*:\s*'(\d{4})'/g)].map((m) => m[1])
  const image = center(src, 'imageCenter')
  if (!ids.length || !image) continue
  const entry = { image, spine: center(src, 'spineCenter') ?? image, disableSpine: /disableSpine\s*:\s*true/.test(src) }
  // variants of a kit (FireflyB1…) share the art: keep one that shows the animation
  for (const id of ids) if (!centers[id] || (centers[id].disableSpine && !entry.disableSpine)) centers[id] = entry
}

await writeFile(out, JSON.stringify(centers, null, 1) + '\n')
console.log(`✓ ${Object.keys(centers).length} characters from ${files.length} files`)
