import { test } from 'node:test'
import assert from 'node:assert/strict'
import { startLive } from '../../src/lib/live-core.ts'
import { mergeWorld } from '../../src/lib/world-patch.ts'

const tick = async () => { for (let i = 0; i < 12; i++) await Promise.resolve() }

/** A screen's subscription with a hand-driven backend. */
function mount(handlers, retryMs = 5) {
  const listeners = new Map()
  const reads = []
  const errors = []
  const live = { reads, errors, shown: undefined }
  live.stop = startLive({
    read: () => new Promise((resolve, reject) => reads.push({ resolve, reject })),
    handlers: () => handlers,
    listen: (name, fn) => { listeners.set(name, fn); return Promise.resolve(() => listeners.delete(name)) },
    show: (state) => { live.shown = state },
    onError: (e) => errors.push(e),
    retryMs,
  })
  live.emit = (name, revision, data) => listeners.get(name)?.({ payload: { revision, data } })
  return live
}

test('a value that was read as null still follows its events', async () => {
  const live = mount({ 'snapshot-changed': () => 'refetch' })
  await tick()
  live.reads[0].resolve({ revision: 0, data: null })
  await tick()
  live.emit('snapshot-changed', 1, 'GetDailyActiveInfoScRsp')
  await tick()
  assert.equal(live.reads.length, 2, 'the event asks for the value again')
  live.reads[1].resolve({ revision: 1, data: { at: 5 } })
  await tick()
  assert.deepEqual(live.shown, { at: 5 })
  live.stop()
})

test('a failed read is retried and the events that came meanwhile are applied', async () => {
  const live = mount({ changed: (_, next) => next })
  await tick()
  live.reads[0].reject(new Error('IPC failure'))
  await tick()
  live.emit('changed', 3, { value: 3 })
  await new Promise((r) => setTimeout(r, 30))
  assert.equal(live.reads.length, 2, 'read again after the failure')
  assert.equal(live.errors.length, 1, 'the failure is reported')
  live.reads[1].resolve({ revision: 2, data: { value: 2 } })
  await tick()
  assert.deepEqual(live.shown, { value: 3 }, 'the newer buffered event wins over the read')
  live.emit('changed', 4, { value: 4 })
  assert.deepEqual(live.shown, { value: 4 }, 'later events apply at once')
  live.stop()
})

test('an answer for the previous account is not shown after an account switch', async () => {
  const live = mount({ changed: (_, next) => next })
  await tick()
  live.emit('account-changed', 2, 222)
  live.reads[0].resolve({ revision: 1, data: { uid: 111 } })
  await tick()
  assert.equal(live.shown, undefined, 'the stale answer is dropped')
  assert.equal(live.reads.length, 2, 'and the state is read again')
  live.reads[1].resolve({ revision: 2, data: { uid: 222 } })
  await tick()
  assert.deepEqual(live.shown, { uid: 222 })
  live.stop()
})

test('several refetch events at once cost one read', async () => {
  const live = mount({ changed: () => 'refetch' })
  await tick()
  live.reads[0].resolve({ revision: 0, data: {} })
  await tick()
  for (let revision = 1; revision <= 5; revision++) live.emit('changed', revision, null)
  await tick()
  assert.equal(live.reads.length, 2)
  live.stop()
})

test('nothing is read or shown after the screen is gone', async () => {
  const live = mount({ changed: (_, next) => next })
  await tick()
  live.reads[0].reject(new Error('IPC failure'))
  await tick()
  live.stop()
  await new Promise((r) => setTimeout(r, 30))
  assert.equal(live.reads.length, 1, 'no retry after stop')
})

test('world-changed replaces only the floors it carries', () => {
  const world = {
    planeId: 1, floorId: 10, entryId: 1, player: null,
    openedChests: { 10: [1], 20: [5] },
    openedAt: { 20: { 5: { x: 1, y: 0, z: 0, rotY: 0, layer: null } } },
    props: { 10: { '5:1': 12 }, 20: { '5:9': 8 } },
  }
  const next = mergeWorld(world, {
    planeId: 2, floorId: 30, entryId: 7, player: { x: 9, y: 0, z: 0, rotY: 0, layer: 1 },
    floors: { 10: { openedChests: [1, 2], openedAt: {}, props: { '5:1': 13 } }, 30: { openedChests: [], openedAt: {}, props: {} } },
  })
  assert.equal(next.floorId, 30)
  assert.equal(next.player.x, 9)
  assert.deepEqual(next.openedChests, { 10: [1, 2], 20: [5], 30: [] })
  assert.deepEqual(next.props, { 10: { '5:1': 13 }, 20: { '5:9': 8 }, 30: {} })
  assert.deepEqual(next.openedAt[20], world.openedAt[20], 'untouched floors are kept')
  assert.deepEqual(world.openedChests[10], [1], 'the previous state is not mutated')
})
