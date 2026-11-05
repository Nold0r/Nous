// Downloads the game tables Nous shows names and values from (Russian,
// StarRailRes index_new/ru) and keeps only the fields the app reads:
// relics, sets, main/sub affixes, stat names, light cones, items, characters.
// Output: src/assets/data/game-data.json, loaded by src/lib/game-tables.ts.
// Run after a game update: npm run game-data
import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const SOURCE = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/index_new/ru/'

const root = path.dirname(fileURLToPath(import.meta.url))
const out = path.join(root, '..', 'src', 'assets', 'data', 'game-data.json')

async function table(name) {
  const res = await fetch(`${SOURCE}${name}.json`)
  if (!res.ok) throw new Error(`${name}: HTTP ${res.status}`)
  return res.json()
}

const map = (obj, fn) => Object.fromEntries(Object.entries(obj).flatMap(([k, v]) => (fn(v) ? [[k, fn(v)]] : [])))
const affixes = (groups) =>
  map(groups, (g) => map(g.affixes, (a) => ({ property: a.property, base: a.base, step: a.step })))

const [relics, relicSets, mainAffixes, subAffixes, properties, lightCones, items, characters] = await Promise.all(
  ['relics', 'relic_sets', 'relic_main_affixes', 'relic_sub_affixes', 'properties', 'light_cones', 'items', 'characters'].map(table),
)

const data = {
  relics: map(relics, (r) => ({
    set: r.set_id,
    slot: r.type,
    rarity: r.rarity,
    mainGroup: r.main_affix_id,
    subGroup: r.sub_affix_id,
    icon: r.icon,
  })),
  relicSets: map(relicSets, (s) => ({ name: s.name, desc: s.desc, icon: s.icon })),
  mainAffixes: affixes(mainAffixes),
  subAffixes: affixes(subAffixes),
  // icon: file name in assets/icons/property (npm run assets)
  properties: map(properties, (p) => p.name && { name: p.name, percent: p.percent, icon: path.basename(p.icon, '.png') }),
  lightCones: map(lightCones, (c) => ({ name: c.name, rarity: c.rarity, path: c.path, icon: c.icon })),
  // quest items and display-only entries never show up in the bag
  items: map(
    items,
    (i) =>
      i.name &&
      i.type !== 'Mission' &&
      i.type !== 'Display' && { name: i.name, rarity: i.rarity, type: i.type, subType: i.sub_type, icon: i.icon },
  ),
  characters: map(characters, (c) => ({ name: c.name, rarity: c.rarity, path: c.path, element: c.element })),
}

await mkdir(path.dirname(out), { recursive: true })
await writeFile(out, JSON.stringify(data))
for (const [k, v] of Object.entries(data)) console.log(`✓ ${k}: ${Object.keys(v).length}`)
