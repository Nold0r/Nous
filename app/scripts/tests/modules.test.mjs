import { test } from 'node:test'
import assert from 'node:assert/strict'
import { ModuleRuntime } from '../../src/features/module-runtime.ts'

function harness(setServices) {
  const errors = []
  const services = []
  const runtime = new ModuleRuntime(setServices ?? (async (value) => { services.push([...value]) }),
    () => ({ services: {}, storage: { get() {}, set() {} }, navigate() {} }), (error) => errors.push(error))
  return { runtime, errors, services }
}
test('screenless background modules share services and stop when disabled', async () => {
  const { runtime, errors, services } = harness()
  const lifecycle = []
  const module = (id) => ({ id, title: id, requires: ['hsr'], start: ({ signal }) => {
    lifecycle.push(`start ${id}`)
    return () => { assert.equal(signal.aborted, true); lifecycle.push(`stop ${id}`) }
  } })
  const a = module('a'), b = module('b')
  await runtime.update([a, b])
  await runtime.update([b])
  assert.deepEqual(services, [['hsr']], 'remaining consumer keeps HSR running')
  assert.deepEqual(lifecycle, ['start a', 'start b', 'stop a'])
  await runtime.update([])
  assert.deepEqual(services, [['hsr'], []])
  assert.deepEqual(lifecycle, ['start a', 'start b', 'stop a', 'stop b'])
  assert.deepEqual(errors, [])
})
test('disabling during async service start prevents stale module work', async () => {
  let release
  const gate = new Promise((resolve) => { release = resolve })
  let started = 0
  const { runtime } = harness(async () => gate)
  const enabling = runtime.update([{ id: 'slow', title: 'slow', requires: ['hsr'], start: () => { started++ } }])
  await Promise.resolve()
  const disabling = runtime.update([])
  release()
  await Promise.all([enabling, disabling])
  assert.equal(started, 0)
})
test('one failed module does not stop the shell or other modules', async () => {
  const { runtime, errors } = harness()
  let healthy = false
  await runtime.update([
    { id: 'bad', title: 'bad', start: () => { throw new Error('fixture') } },
    { id: 'good', title: 'good', start: () => { healthy = true } },
  ])
  assert.equal(healthy, true)
  assert.equal(errors.length, 1)
  await runtime.update([])
})
