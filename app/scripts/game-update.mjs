// Moves Nous to a new game version in one run: everything that used to be
// done by hand after an update (docs/Руководства/Обновление игры.md).
//
//   npm run game-update                    all steps
//   npm run game-update -- --only maps     some steps (comma-separated)
//   npm run game-update -- --skip tables
//   npm run game-update -- --game "E:/…/Star Rail Games" --work C:/nous-data
//
// Steps, in order:
//   reliquary  vendor/reliquary to upstream main (decryption key and protos of
//              the new version; without it capture sees nothing)
//   tables     game tables, icons and fonts, character art framing (StarRailRes,
//              HSR Optimizer)
//   datamine   sparse clone of TurnBasedGameData, updated to its latest commit
//   textures   minimap textures from the game's own files (AnimeStudio, read only)
//   maps       map pictures and maps.json, then the marks
//   check      Rust build and tests, module tests, replay of recorded logins
//
// Work files (datamine, AnimeStudio, asset map, textures; several GB) live in
// --work, default C:/nous-data or NOUS_DATA_DIR, not in the repository.
import { spawnSync } from 'node:child_process'
import { createReadStream, existsSync, mkdirSync, readdirSync, readFileSync, linkSync, copyFileSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import readline from 'node:readline'
import { fileURLToPath } from 'node:url'

const APP = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')
const ROOT = path.join(APP, '..')
const STEPS = ['reliquary', 'tables', 'datamine', 'textures', 'maps', 'check']
const DATAMINE = 'https://gitlab.com/Dimbreath/turnbasedgamedata.git'
const DATAMINE_PATHS = [
  '/ExcelOutput/',
  '/TextMap/TextMapRU*.json',
  '/TextMap/TextMapMainRU*.json',
  '/Config/LevelOutput/RuntimeFloor/',
  '/Config/LevelOutput/SharedRuntimeGroup/',
  '/Config/LevelOutput/RuntimeGroup/',
  '/Config/LevelOutput/Map/',
  '/Config/LevelOutput_Baked/Floor/',
]
const ANIMESTUDIO = 'https://github.com/Escartem/AnimeStudio.git'
// containers of the textures the map is built from (map-textures.py reads them
// under <textures>/assets/asbres/spriteoutput/mappics/minimap)
const TEXTURE_CONTAINERS = ['spriteoutput/mappics/minimap/']

function args() {
  const out = { only: null, skip: new Set(), game: process.env.NOUS_GAME_DIR, work: process.env.NOUS_DATA_DIR || 'C:/nous-data' }
  const a = process.argv.slice(2)
  for (let i = 0; i < a.length; i++) {
    const v = a[i + 1]
    if (a[i] === '--only') out.only = new Set(v.split(','))
    else if (a[i] === '--skip') out.skip = new Set(v.split(','))
    else if (a[i] === '--game') out.game = v
    else if (a[i] === '--work') out.work = v
    else throw new Error(`unknown argument ${a[i]}`)
    i++
  }
  for (const s of [...(out.only ?? []), ...out.skip]) if (!STEPS.includes(s)) throw new Error(`unknown step ${s}; steps: ${STEPS.join(', ')}`)
  return out
}

function run(cmd, argv, opts = {}) {
  console.log(`  $ ${cmd} ${argv.join(' ')}`)
  const r = spawnSync(cmd, argv, { stdio: opts.capture ? 'pipe' : 'inherit', encoding: 'utf8', shell: opts.shell ?? false, cwd: opts.cwd ?? ROOT })
  if (r.status !== 0 && !opts.allowFail) throw new Error(`${cmd} ${argv.join(' ')} exited with ${r.status}${r.stderr ? `\n${r.stderr}` : ''}`)
  return r
}
const git = (cwd, ...argv) => run('git', argv, { cwd, capture: true }).stdout.trim()
const npm = (...argv) => run('npm', ['run', ...argv], { cwd: APP, shell: true })

/** The game folder: --game, NOUS_GAME_DIR, the running game, usual install places. */
function findGame(given) {
  const ok = (d) => d && existsSync(path.join(d, 'StarRail_Data'))
  if (given) {
    if (!ok(given)) throw new Error(`no StarRail_Data in ${given}`)
    return given
  }
  const running = spawnSync('powershell', ['-NoProfile', '-Command', '(Get-Process StarRail -ErrorAction SilentlyContinue | Select-Object -First 1).Path'], { encoding: 'utf8' })
  const exe = running.stdout?.trim()
  if (exe && ok(path.dirname(exe))) return path.dirname(exe)
  const places = []
  for (const drive of 'CDEFG') {
    places.push(`${drive}:/Program Files/HoYoPlay/games/Star Rail Games`, `${drive}:/HoYoPlay/games/Star Rail Games`,
      `${drive}:/SteamLibrary/steamapps/common/Star Rail Games`, `${drive}:/Program Files (x86)/Steam/steamapps/common/Star Rail Games`,
      `${drive}:/Star Rail/Games`, `${drive}:/Games/Star Rail Games`)
  }
  const found = places.find(ok)
  if (!found) throw new Error('game folder not found: pass --game "<folder with StarRail_Data>" or set NOUS_GAME_DIR')
  return found
}

function gameVersion(game) {
  const ini = path.join(game, 'config.ini')
  return existsSync(ini) ? (readFileSync(ini, 'utf8').match(/game_version=(\S+)/)?.[1] ?? 'unknown') : 'unknown'
}

// ---- steps ----

function reliquary() {
  const dir = path.join(ROOT, 'vendor', 'reliquary')
  if (!existsSync(path.join(dir, '.git'))) run('git', ['submodule', 'update', '--init', 'vendor/reliquary'])
  git(dir, 'fetch', '--quiet', 'origin')
  const before = git(dir, 'rev-parse', 'HEAD')
  const after = git(dir, 'rev-parse', 'origin/HEAD')
  if (before === after) return console.log('  reliquary is already at upstream main')
  console.log(git(dir, 'log', '--oneline', `${before}..${after}`))
  git(dir, 'checkout', '--quiet', after)
  console.log(`  reliquary ${before.slice(0, 9)} -> ${after.slice(0, 9)}; commit the submodule pointer`)
}

function tables() {
  npm('game-data')
  npm('assets')
  npm('spine-centers')
}

function datamine(work) {
  const dir = path.join(work, 'tbgd')
  if (!existsSync(path.join(dir, '.git'))) {
    mkdirSync(work, { recursive: true })
    run('git', ['clone', '--filter=blob:none', '--no-checkout', '--depth', '1', DATAMINE, dir])
    git(dir, 'sparse-checkout', 'init', '--no-cone')
  } else {
    git(dir, 'fetch', '--quiet', '--depth', '1', 'origin')
  }
  writeFileSync(path.join(dir, '.git', 'info', 'sparse-checkout'), DATAMINE_PATHS.join('\n') + '\n')
  run('git', ['checkout', '--quiet', '-B', 'main', 'origin/HEAD'], { cwd: dir })
  console.log(`  datamine: ${git(dir, 'log', '-1', '--format=%cs %s')}`)
  return dir
}

function animeStudio(work) {
  const dir = path.join(work, 'AnimeStudio')
  const exe = path.join(dir, 'AnimeStudio.CLI', 'bin', 'Release', 'net10.0-windows', 'AnimeStudio.CLI.exe')
  if (existsSync(exe)) return exe
  // built from source: Windows Defender flags the released build
  if (!existsSync(dir)) run('git', ['clone', '--depth', '1', ANIMESTUDIO, dir])
  run('dotnet', ['restore', 'AnimeStudio.CLI', '-s', 'https://api.nuget.org/v3/index.json'], { cwd: dir })
  run('dotnet', ['build', 'AnimeStudio.CLI', '-c', 'Release', '-f', 'net10.0-windows', '--no-restore'], { cwd: dir })
  return exe
}

/** Source files of the assets whose container matches, read line by line from
 *  AnimeStudio's indented JSON map (gigabytes; too big to parse whole). */
async function blocksFor(mapFile, containers) {
  const out = new Set()
  let container = null, source = null
  const lines = readline.createInterface({ input: createReadStream(mapFile, 'utf8'), crlfDelay: Infinity })
  const value = (line) => JSON.parse(line.slice(line.indexOf(':') + 1).trim().replace(/,$/, ''))
  for await (const line of lines) {
    const t = line.trimStart()
    if (t.startsWith('"Container"')) container = value(t)
    else if (t.startsWith('"Source"')) source = value(t)
    else if (t.startsWith('}')) {
      if (container && source && containers.some((c) => container.toLowerCase().includes(c))) out.add(source)
      container = source = null
    }
  }
  return [...out]
}

async function textures(work, game) {
  const version = gameVersion(game)
  const exe = animeStudio(work)
  const asb = path.join(game, 'StarRail_Data', 'StreamingAssets', 'Asb', 'Windows')
  const mapDir = path.join(work, 'assetmap', version)
  const out = path.join(work, 'textures', version)
  if (existsSync(path.join(out, 'assets', 'asbres', 'spriteoutput', 'mappics', 'minimap'))) {
    console.log(`  textures of ${version} are already exported: ${out}`)
    return out
  }
  const mapFile = () => existsSync(mapDir) && readdirSync(mapDir).find((f) => f.endsWith('.json'))
  if (!mapFile()) {
    // ~10 minutes, ~3 GB; kept per game version
    mkdirSync(mapDir, { recursive: true })
    run(exe, [asb, mapDir, '--game', 'SR', '--map_op', 'AssetMap', '--map_type', 'JSON', '--silent'])
  }
  if (!mapFile()) throw new Error(`AnimeStudio wrote no asset map into ${mapDir}`)
  const sources = await blocksFor(path.join(mapDir, mapFile()), TEXTURE_CONTAINERS)
  if (!sources.length) throw new Error('no minimap textures in the asset map')
  console.log(`  ${sources.length} game files hold the minimap textures`)
  // AnimeStudio's --containers filter does not work: export from a folder that
  // holds only these files
  const blocks = path.join(work, 'blocks', version)
  rmSync(blocks, { recursive: true, force: true })
  mkdirSync(blocks, { recursive: true })
  for (const src of sources) {
    const file = path.isAbsolute(src) ? src : path.join(asb, src)
    const to = path.join(blocks, path.basename(file))
    try { linkSync(file, to) } catch { copyFileSync(file, to) }
  }
  run(exe, [blocks, out, '--game', 'SR', '--types', 'Texture2D', '--group_assets', 'ByContainer', '--silent'])
  return out
}

function maps(tbgd, tex) {
  const before = new Set(Object.keys(JSON.parse(readFileSync(path.join(APP, 'modules', 'map', 'maps.json'), 'utf8'))))
  npm('map-textures', '--', tbgd, tex)
  npm('map-markers', '--', tbgd)
  const after = JSON.parse(readFileSync(path.join(APP, 'modules', 'map', 'maps.json'), 'utf8'))
  const added = Object.keys(after).filter((f) => !before.has(f))
  const removed = [...before].filter((f) => !(f in after))
  console.log(`  maps: ${Object.keys(after).length} floors; new ${added.length}, gone ${removed.length}`)
  for (const f of added) console.log(`    + ${f} ${after[f].name}`)
  for (const f of removed) console.log(`    - ${f}`)
}

function check() {
  run('cargo', ['test', '--workspace', '--all-targets'])
  npm('test:modules')
  npm('test:data')
}

// ---- main ----

let opt
try {
  opt = args()
} catch (e) {
  console.error(e.message)
  process.exit(2)
}
const want = (s) => (opt.only ? opt.only.has(s) : true) && !opt.skip.has(s)
const work = path.resolve(opt.work)
let game = null, tbgd = path.join(work, 'tbgd'), tex = null
const step = async (name, fn) => {
  if (!want(name)) return
  console.log(`\n== ${name}`)
  await fn()
}

try {
  if (want('textures') || want('maps')) {
    game = findGame(opt.game)
    console.log(`game ${gameVersion(game)}: ${game}`)
  }
  await step('reliquary', reliquary)
  await step('tables', tables)
  await step('datamine', () => { tbgd = datamine(work) })
  await step('textures', async () => { tex = await textures(work, game) })
  await step('maps', () => {
    tex ??= path.join(work, 'textures', gameVersion(game))
    if (!existsSync(path.join(tbgd, 'ExcelOutput'))) throw new Error(`no datamine in ${tbgd}: run the datamine step`)
    if (!existsSync(tex)) throw new Error(`no textures in ${tex}: run the textures step`)
    maps(tbgd, tex)
  })
  await step('check', check)
  console.log('\nDone. Review `git status`, then add the patch notes to CHANGELOG.md (Unreleased).')
} catch (e) {
  console.error(`\nfailed: ${e.message}`)
  process.exit(1)
}
