import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { formatStat } from '../../src/lib/format-stat.ts'

// Relic values are sums of decimal constants (base + step × n). The game cuts
// them to one decimal of a percent; the cut must be taken of the decimal value,
// not of its binary approximation (0.162 * 1000 is 161.99999999999997).
const tables = JSON.parse(readFileSync(new URL('../../src/assets/data/game-data.json', import.meta.url), 'utf8'))

const SCALE = 10n ** 12n
const decimal = (n) => {
  const [whole, fraction = ''] = String(n).split('.')
  return BigInt(whole) * SCALE + BigInt((fraction + '0'.repeat(12)).slice(0, 12))
}
const exactPercent = (scaled) => `${(Number((scaled * 1000n) / SCALE) / 10).toLocaleString('ru-RU')}%`

test('percent sub stats are cut like the exact decimal value', () => {
  const wrong = []
  let checked = 0
  for (const affixes of Object.values(tables.subAffixes)) {
    for (const d of Object.values(affixes)) {
      if (!tables.properties[d.property]?.percent) continue
      for (let rolls = 1; rolls <= 6; rolls++) {
        for (let steps = 0; steps <= 2 * rolls; steps++) {
          checked++
          const shown = formatStat({ property: d.property, value: rolls * d.base + steps * d.step, percent: true })
          const exact = exactPercent(decimal(d.base) * BigInt(rolls) + decimal(d.step) * BigInt(steps))
          if (shown !== exact) wrong.push(`${d.property} ${rolls}/${steps}: ${shown} instead of ${exact}`)
        }
      }
    }
  }
  assert.ok(checked > 1000, `checked ${checked} combinations`)
  assert.deepEqual(wrong, [])
})

test('percent main stats at every level are cut like the exact decimal value', () => {
  const wrong = []
  for (const affixes of Object.values(tables.mainAffixes)) {
    for (const d of Object.values(affixes)) {
      if (!tables.properties[d.property]?.percent) continue
      for (let level = 0; level <= 15; level++) {
        const shown = formatStat({ property: d.property, value: d.base + d.step * level, percent: true })
        const exact = exactPercent(decimal(d.base) + decimal(d.step) * BigInt(level))
        if (shown !== exact) wrong.push(`${d.property} +${level}: ${shown} instead of ${exact}`)
      }
    }
  }
  assert.deepEqual(wrong, [])
})

test('flat stats and speed keep their format', () => {
  assert.equal(formatStat({ property: 'HPDelta', value: 135.9, percent: false }), '135')
  assert.equal(formatStat({ property: 'SpeedDelta', value: 2.6, percent: false }), '2,6')
  assert.equal(formatStat({ property: 'SpeedDelta', value: 2.3 + 2.3, percent: false }), '4,6')
})
