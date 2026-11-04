import { test } from 'node:test'
import assert from 'node:assert/strict'
import { ModuleRuntime } from '../../src/features/module-runtime.ts'

function harness() {
  const errors = []
  const runtime = new ModuleRuntime(
    () => ({ sdk: { read: async () => null }, storage: { get() {}, set() {} }, navigate() {} }), (error) => errors.push(error))
  return { runtime, errors }
}
test('background work runs while a module is enabled and stops when it is disabled', () => {
  const { runtime, errors } = harness()
  const lifecycle = []
  const module = (id) => ({ id, title: id, start: ({ signal }) => {
    lifecycle.push(`start ${id}`)
    return () => { assert.equal(signal.aborted, true); lifecycle.push(`stop ${id}`) }
  } })
  const a = module('a'), b = module('b')
  runtime.update([a, b])
  runtime.update([b])
  assert.deepEqual(lifecycle, ['start a', 'start b', 'stop a'])
  runtime.update([])
  assert.deepEqual(lifecycle, ['start a', 'start b', 'stop a', 'stop b'])
  assert.deepEqual(errors, [])
})
test('changed settings restart a module with the new values', () => {
  const { runtime } = harness()
  const seen = []
  const module = { id: 'm', title: 'm', settings: { defaults: { zoom: 1 } }, start: ({ settings }) => { seen.push(settings.zoom) } }
  runtime.update([module], {})
  runtime.update([module], { m: { zoom: 2 } })
  assert.deepEqual(seen, [1, 2])
})
test('one failed module does not stop the shell or other modules', () => {
  const { runtime, errors } = harness()
  let healthy = false
  runtime.update([
    { id: 'bad', title: 'bad', start: () => { throw new Error('fixture') } },
    { id: 'good', title: 'good', start: () => { healthy = true } },
  ])
  assert.equal(healthy, true)
  assert.equal(errors.length, 1)
  runtime.update([])
})
