// Executes the actual TS hook with deterministic React/Tauri substitutes.
// No browser, native IPC, network, or application storage is used.
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { stripTypeScriptTypes } from 'node:module'
import vm from 'node:vm'
import assert from 'node:assert/strict'

const here = dirname(fileURLToPath(import.meta.url))
const root = process.env.NOUS_AUDIT_ROOT ?? resolve(here, '../..')
const source = readFileSync(resolve(root, 'app/src/lib/live.ts'), 'utf8')
const executable = stripTypeScriptTypes(source.replace(/^import .*\r?\n/gm, '').replace('export function useLive', 'function useLive'))
const tick = async () => { for (let i = 0; i < 12; i++) await Promise.resolve() }

function mount(initial, handlers) {
  const listeners = new Map(), reads = [], errors = []
  let shown
  const context = vm.createContext({
    Promise, console: { error: e => errors.push(String(e)) }, isTauri: () => true,
    useState: () => [null, value => { shown = value }],
    useRef: current => ({ current }), useEffect: fn => fn(),
    listen: (name, fn) => { listeners.set(name, fn); return Promise.resolve(() => listeners.delete(name)) },
  })
  vm.runInContext(executable, context)
  const read = () => new Promise((resolve, reject) => reads.push({ resolve, reject }))
  context.useLive(read, handlers)
  return {
    reads, errors, get shown() { return shown },
    emit: (name, revision, data) => listeners.get(name)({ payload: { revision, data } }),
    async start() { await tick(); reads[0].resolve(initial); await tick() },
  }
}

const tests = {
  async snapshot_initially_null_never_refetches() {
    const h = mount({ revision: 0, data: null }, { 'snapshot-changed': () => 'refetch' })
    await h.start()
    h.emit('snapshot-changed', 1, 'GetDailyActiveInfoScRsp')
    await tick()
    assert.equal(h.reads.length, 2, 'first snapshot event must read the newly available data')
  },
  async failed_initial_read_buffers_forever() {
    const h = mount(null, { changed: (_, next) => next })
    await tick()
    h.reads[0].reject(new Error('temporary IPC failure'))
    await tick()
    h.emit('changed', 1, { value: 'fresh' })
    await tick()
    assert.equal(h.shown?.value, 'fresh', 'a later full-state event should recover after failed initial read')
  },
  async account_switch_accepts_old_inflight_read() {
    const h = mount(null, { changed: (_, next) => next })
    await tick()
    h.emit('account-changed', 2, 222)
    h.reads[0].resolve({ revision: 1, data: { uid: 111 } })
    await tick()
    assert.notEqual(h.shown?.uid, 111, 'a response started before account-changed must be discarded')
  },
  async account_switch_leaves_old_account_visible() {
    const h = mount({ revision: 1, data: { uid: 111 } }, {})
    await h.start()
    h.emit('account-changed', 2, 222)
    assert.notEqual(h.shown?.uid, 111, 'old account data must be invalidated immediately')
  },
  async failed_refetch_stops_further_updates() {
    const h = mount({ revision: 1, data: { value: 1 } }, {
      reset: () => 'refetch', changed: (_, next) => next,
    })
    await h.start()
    h.emit('reset', 2, null)
    h.reads[1].reject(new Error('temporary IPC failure'))
    await tick()
    h.emit('changed', 3, { value: 3 })
    await tick()
    assert.equal(h.shown?.value, 3, 'a failed refetch must not buffer all future events forever')
  },
  async normal_read_and_out_of_order_event_control() {
    const h = mount({ revision: 5, data: { value: 5 } }, { changed: (_, next) => next })
    await h.start()
    h.emit('changed', 4, { value: 4 })
    h.emit('changed', 6, { value: 6 })
    assert.equal(h.shown?.value, 6)
  },
}

for (const [file, hook, event, oldState, newState] of [
  ['game.ts', 'useGameStatus', 'game-status', { running: false, path: 'C:/test/StarRail.exe' }, { running: true, path: 'C:/test/StarRail.exe' }],
  ['sniffer.ts', 'useSnifferStatus', 'sniffer-status', { state: 'starting' }, { state: 'connected' }],
  ['veritas.ts', 'useVeritasStatus', 'veritas-status', { enabled: false, connected: false }, { enabled: true, connected: true }],
]) {
  tests[`${hook}_old_read_must_not_overwrite_new_event`] = async () => {
    const listeners = new Map(), reads = []
    let shown
    const code = stripTypeScriptTypes(readFileSync(resolve(root, 'app/src/lib', file), 'utf8')
      .replace(/^import .*\r?\n/gm, '').replace(/^export /gm, ''))
    const context = vm.createContext({ Promise, console, isTauri: () => true,
      useEffect: fn => fn(), useState: () => [null, v => { shown = v }],
      invoke: () => new Promise(resolve => reads.push(resolve)),
      listen: (name, fn) => { listeners.set(name, fn); return Promise.resolve(() => {}) },
    })
    vm.runInContext(code, context)
    context[hook]()
    listeners.get(event)({ payload: newState })
    if (file === 'veritas.ts') { reads[1](newState); await tick() }
    reads[0](oldState)
    await tick()
    assert.equal(JSON.stringify(shown), JSON.stringify(newState), 'a late initial read rolled the state back')
  }
}

const results = []
for (const [name, run] of Object.entries(tests)) {
  try { await run(); results.push({ name, status: 'PASS' }) }
  catch (e) { results.push({ name, status: 'FAIL', message: e.message }) }
}
writeFileSync(resolve(here, process.env.NOUS_AUDIT_FRONTEND_RESULT ?? 'results/frontend.json'), JSON.stringify(results, null, 2) + '\n')
for (const r of results) console.log(`${r.status} ${r.name}${r.message ? ': ' + r.message : ''}`)
process.exitCode = results.some(r => r.status === 'FAIL') ? 1 : 0
