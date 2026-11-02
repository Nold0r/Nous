// Downloads game assets from StarRailRes (listed in scripts/assets.json):
// - icons, normalized so every icon of a group looks the same size: trim
//   transparent padding, fit into a square with equal margins;
// - fonts, subset to Latin + Cyrillic and converted to woff2 (12 MB -> ~100 KB).
// Run: npm run assets
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import sharp from 'sharp'
import subsetFont from 'subset-font'

const SIZE = 128 // icon canvas
const transparent = { r: 0, g: 0, b: 0, alpha: 0 }

const root = path.dirname(fileURLToPath(import.meta.url))
const assets = path.join(root, '..', 'src', 'assets')
const config = JSON.parse(await readFile(path.join(root, 'assets.json'), 'utf8'))

async function download(srcPath, id, label) {
  const res = await fetch(config.source + srcPath.replace('{id}', id))
  if (!res.ok) {
    console.error(`✗ ${label} (${id}): HTTP ${res.status}`)
    process.exitCode = 1
    return null
  }
  return Buffer.from(await res.arrayBuffer())
}

for (const [group, { path: srcPath, content, icons }] of Object.entries(config.groups)) {
  const outDir = path.join(assets, 'icons', group)
  await mkdir(outDir, { recursive: true })

  for (const [name, id] of Object.entries(icons)) {
    const input = await download(srcPath, id, `${group}/${name}`)
    if (!input) continue

    const trimmed = await sharp(input).trim({ threshold: 1 }).toBuffer()
    const fitted = await sharp(trimmed).resize(content, content, { fit: 'contain', background: transparent }).toBuffer()
    const pad = (SIZE - content) / 2
    await sharp(fitted)
      .extend({ top: pad, bottom: pad, left: pad, right: pad, background: transparent })
      .png()
      .toFile(path.join(outDir, `${name}.png`))

    console.log(`✓ ${group}/${name} (${id})`)
  }
}

// Basic Latin, Latin-1, Cyrillic, general punctuation, arrows, a few symbols
const charset = [
  [0x20, 0x7e],
  [0xa0, 0xff],
  [0x400, 0x4ff],
  [0x2010, 0x206f],
  [0x2190, 0x21ff],
  [0x2116, 0x2116],
  [0x25a0, 0x25ff],
]
  .flatMap(([from, to]) => Array.from({ length: to - from + 1 }, (_, i) => String.fromCodePoint(from + i)))
  .join('')

const fontDir = path.join(assets, 'fonts')
await mkdir(fontDir, { recursive: true })
for (const [name, id] of Object.entries(config.fonts.fonts)) {
  const input = await download(config.fonts.path, id, `font/${name}`)
  if (!input) continue
  const woff2 = await subsetFont(input, charset, { targetFormat: 'woff2' })
  await writeFile(path.join(fontDir, `${name}.woff2`), woff2)
  console.log(`✓ font/${name} (${id}): ${(input.length / 1e6).toFixed(1)} MB -> ${(woff2.length / 1e3).toFixed(0)} KB`)
}
