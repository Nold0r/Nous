import assert from 'node:assert/strict'
import test from 'node:test'
import { characterValue, damageCategories, dpav, effective, projectedAv, share, sortedAvatars, themeVariables } from './presentation.ts'

const avatars = [
  { id: 1, damage: 100, overkill: 30, byType: { Basic: 40, Skill: 60 }, properties: {} },
  { id: 2, damage: 300, overkill: 400, byType: { Skill: 200, UnknownType: 100, Invalid: NaN, Negative: -1 }, properties: {} },
]
const battle = { avatars, actionValue: 20, totalDamage: 400, turnHistory: [{ avatarsTurnDamage: [0, 300] }, { avatarsTurnDamage: [100, 0] }, { avatarsTurnDamage: [0, 0] }] }

test('zero AV and invalid values never produce invalid display metrics', () => {
  assert.equal(dpav(100, 0), 0)
  assert.equal(dpav(100, -1), 0)
  assert.equal(dpav(Infinity, 20), 0)
  assert.equal(share(100, 0), 0)
  assert.equal(effective(avatars[1]), 0)
})

test('absolute action AV matches upstream and does not invent missing stats', () => {
  assert.equal(projectedAv(231.5, 13.5), 245)
  assert.equal(projectedAv(231.5, -50), 231.5)
  assert.equal(projectedAv(231.5, 0), 231.5)
  assert.equal(projectedAv(231.5, undefined), null)
  assert.equal(projectedAv(231.5, Infinity), null)
  assert.equal(projectedAv(NaN, 5), null)
})

test('team aggregation preserves unknown damage types and excludes invalid contributions', () => {
  assert.deepEqual(damageCategories(battle, null), [['Skill', 260], ['UnknownType', 100], ['Basic', 40]])
  assert.deepEqual(damageCategories(battle, 1), [['Skill', 60], ['Basic', 40]])
  assert.deepEqual(damageCategories(battle, 999), [])
})

test('sorting does not mutate lineup or corrupt per-character turn association', () => {
  const sorted = sortedAvatars(battle, 'dpav')
  assert.deepEqual(sorted.map((avatar) => avatar.id), [2, 1])
  assert.deepEqual(avatars.map((avatar) => avatar.id), [1, 2])
  assert.equal(characterValue(sorted[0], 'turns', battle), '1')
  assert.equal(characterValue(sorted[1], 'effective', battle), '70')
})

test('widgets keep the Nous palette: only opacity and font sizes vary', () => {
  const variables = themeVariables({ theme: 'warm', opacity: .8, fontSizes: {}, legendSize: 'body' })
  assert.equal(variables['--gold'], undefined)
  assert.equal(variables['--vt-surface'], undefined)
  assert.equal(variables['--vt-opacity'], .8)
})
