import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { stripTypeScriptTypes } from 'node:module'
import vm from 'node:vm'
import assert from 'node:assert/strict'
const here = dirname(fileURLToPath(import.meta.url))
const sdk = process.env.NOUS_AUDIT_ROOT ?? resolve(here, '../../../../Nous-sdk')
const src = readFileSync(resolve(sdk, 'app/src/features/module-runtime.ts'), 'utf8')
const code = stripTypeScriptTypes(src.replace(/^import .*\r?\n/gm, '').replace('export class ModuleRuntime', 'class ModuleRuntime'))
const context = vm.createContext({ Promise, AbortController, Map, Set })
vm.runInContext(code + '\nglobalThis.ModuleRuntime = ModuleRuntime', context)
const { ModuleRuntime } = context
const noContext = () => ({ services: {}, storage: {}, navigate() {} })
const tests = {
  async unavailable_hsr_should_not_block_independent_module() {
    let independent = false
    const rt = new ModuleRuntime(async () => { throw new Error('HSR unavailable') }, noContext, () => {})
    await rt.update([
      { id: 'hsr', title: 'HSR', requires: ['hsr'] },
      { id: 'notes', title: 'Notes', start: () => { independent = true } },
    ])
    assert.equal(independent, true, 'a failure of HSR prevented a module with no service dependencies from starting')
  },
  async reentrant_disable_during_start_must_stop_started_module() {
    let signal, stopped = false
    const rt = new ModuleRuntime(async () => {}, noContext, () => {})
    await rt.update([{ id: 'm', title: 'm', start: c => {
      signal = c.signal
      void rt.update([])
      return () => { stopped = true }
    } }])
    // Drain the queued update scheduled by start.
    await Promise.resolve(); await Promise.resolve()
    assert.equal(signal.aborted && stopped, true, 'update([]) during start missed the not-yet-registered module')
  },
  async navigation_allowlist_must_compare_origins_control_and_attack() {
    const file = readFileSync(resolve(sdk, 'app/src-tauri/src/veritas/overlay_host.rs'), 'utf8')
    assert.ok(file.includes('starts_with(&allowed)'), 'audit assertion needs updating after native predicate changes')
    const allowed = 'http://tauri.localhost'
    assert.equal('http://tauri.localhost/index.html'.startsWith(allowed), true)
    // Exact operation used by the Rust navigation callback, with a distinct origin.
    const attacker = 'http://tauri.localhost.attacker.invalid/'
    assert.equal(new URL(attacker).origin === allowed, false)
    assert.equal(attacker.startsWith(allowed), false, 'native starts_with accepts a different hostname as trusted')
  },
  async ordinary_module_lifecycle_control() {
    let starts = 0, stops = 0
    const rt = new ModuleRuntime(async () => {}, noContext, () => {})
    await rt.update([{ id: 'm', title: 'm', start: () => { starts++; return () => { stops++ } } }])
    await rt.update([])
    assert.equal(starts, 1); assert.equal(stops, 1)
  },
}
const results = []
for (const [name, run] of Object.entries(tests)) {
  try { await run(); results.push({ name, status: 'PASS' }) }
  catch (e) { results.push({ name, status: 'FAIL', message: e.message }) }
}
writeFileSync(resolve(here, 'results/modules.json'), JSON.stringify(results, null, 2) + '\n')
for (const r of results) console.log(`${r.status} ${r.name}${r.message ? ': ' + r.message : ''}`)
process.exitCode = results.some(r => r.status === 'FAIL') ? 1 : 0
