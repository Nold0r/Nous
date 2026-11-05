// Downloads the game tables Nous shows names and values from (Russian,
// StarRailRes index_new/ru) and keeps only the fields the app reads:
// relics, sets, main/sub affixes, stat names, light cones, items, characters.
// Output: src/assets/data/game-data.json, loaded by src/lib/game-tables.ts;
// game-data-characters.json (base stats, light cone stats, traces, skills,
// eidolons), loaded only by the characters screen (useCharacterTables).
// Run after a game update: npm run game-data
import { mkdir, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const SOURCE = 'https://raw.githubusercontent.com/Mar-7th/StarRailRes/master/index_new/ru/'

const root = path.dirname(fileURLToPath(import.meta.url))
const dataDir = path.join(root, '..', 'src', 'assets', 'data')

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
  // properties: unconditional stats of the 2- and 4-piece bonuses
  relicSets: map(relicSets, (s) => ({ name: s.name, desc: s.desc, icon: s.icon, properties: s.properties })),
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

// ---------- characters screen ----------

const [promotions, lcPromotions, lcRanks, trees, skills, ranks, elements, paths] = await Promise.all(
  [
    'character_promotions',
    'light_cone_promotions',
    'light_cone_ranks',
    'character_skill_trees',
    'character_skills',
    'character_ranks',
    'elements',
    'paths',
  ].map(table),
)

const stat = (v) => v && [v.base, v.step]
// trace points: character id -> "Point01" -> what it is. Points with a skill
// (basic attack, skill, ultimate, talent, memosprite) carry its id; stat
// traces carry their bonus.
const traceTree = {}
for (const t of Object.values(trees)) {
  const owner = t.id.slice(0, -3)
  traceTree[owner] ??= {}
  traceTree[owner][t.anchor] = {
    maxLevel: t.max_level,
    name: t.name,
    icon: t.icon,
    skill: t.level_up_skills[0]?.id,
    properties: t.levels[0]?.properties ?? [],
  }
}
const usedSkills = new Set(Object.values(traceTree).flatMap((t) => Object.values(t).flatMap((p) => p.skill ?? [])))

const characterData = {
  // ascension -> [base, step per level]; level 1 = base
  promotions: map(promotions, (p) =>
    p.values.map((v) => ({
      hp: stat(v.hp),
      atk: stat(v.atk),
      def: stat(v.def),
      spd: stat(v.spd),
      critRate: stat(v.crit_rate),
      critDmg: stat(v.crit_dmg),
    })),
  ),
  lightConePromotions: map(lcPromotions, (p) => p.values.map((v) => ({ hp: stat(v.hp), atk: stat(v.atk), def: stat(v.def) }))),
  // superimposition 1..5 -> unconditional passive stats
  lightConeRanks: map(lcRanks, (r) => ({ skill: r.skill, desc: r.desc, params: r.params, properties: r.properties })),
  traces: traceTree,
  skills: map(skills, (s) =>
    usedSkills.has(s.id) && { name: s.name, type: s.type, typeText: s.type_text, maxLevel: s.max_level, icon: s.icon },
  ),
  // character id -> eidolons 1..6
  eidolons: Object.values(ranks).reduce((acc, r) => {
    const owner = r.id.slice(0, -2)
    ;(acc[owner] ??= [])[r.rank - 1] = { name: r.name, desc: r.desc, icon: r.icon, skills: r.level_up_skills }
    return acc
  }, {}),
  elements: map(elements, (e) => ({ name: e.name, color: e.color, icon: e.icon })),
  paths: map(paths, (p) => ({ name: p.name, icon: p.icon })),
  characterIcons: map(characters, (c) => ({ icon: c.icon, portrait: c.portrait })),
}

await mkdir(dataDir, { recursive: true })
await writeFile(path.join(dataDir, 'game-data.json'), JSON.stringify(data))
await writeFile(path.join(dataDir, 'game-data-characters.json'), JSON.stringify(characterData))
for (const [k, v] of Object.entries({ ...data, ...characterData })) console.log(`✓ ${k}: ${Object.keys(v).length}`)
