// Read-only browser fixture: no native IPC, DLL installation, or running game.
import { resolve } from 'node:path'
import { mkdir } from 'node:fs/promises'
import assert from 'node:assert/strict'

process.env.PLAYWRIGHT_BROWSERS_PATH ??= resolve('../target/playwright')
const { chromium } = await import('playwright')
const browser = await chromium.launch({ headless: true })
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } })
  const errors = []
  page.on('pageerror', (error) => errors.push(String(error)))
  await page.goto(`${process.env.NOUS_UI_URL ?? 'http://127.0.0.1:5174/'}#/settings`)
  await page.getByRole('heading', { name: 'Урон в реальном времени (Veritas)', exact: true }).waitFor()
  assert.equal(await page.getByRole('heading', { name: 'Оверлей Nous' }).count(), 0, 'overlay settings live inside the Veritas section')
  await page.getByRole('tab', { name: 'Окно урона', exact: true }).waitFor()
  await page.getByRole('combobox', { name: 'Положение', exact: true }).selectOption('bottom-left')
  await page.getByRole('tab', { name: 'Вид', exact: true }).click()
  await page.getByRole('combobox', { name: 'Акцентный цвет', exact: true }).selectOption('light')
  await page.getByRole('tab', { name: 'Экспорт', exact: true }).click()
  await page.getByText('Дополнительно (настройки плагина)').click()
  assert.equal(await page.getByRole('button', { name: 'Загрузить настройки плагина' }).isDisabled(), true)
  for (const removed of ['Горячие', 'Бета', 'Defender', 'Собственные цвета']) assert.equal(await page.getByText(removed).count(), 0, `${removed} removed`)
  await page.getByRole('tab', { name: 'Окно урона', exact: true }).click()
  await mkdir('../target/ui-evidence', { recursive: true })
  await page.locator('.vt-settings').locator('xpath=..').screenshot({ path: '../target/ui-evidence/veritas-settings-browser.png' })
  await page.evaluate(async () => {
    const { default: React } = await import('/node_modules/.vite/deps/react.js')
    const { default: ReactDOM } = await import('/node_modules/.vite/deps/react-dom_client.js')
    const { BattleView } = await import('/src/veritas/BattleView.tsx')
    const { defaultSettings } = await import('/src/veritas/types.ts')
    document.getElementById('root').style.display = 'none'
    const fixture = document.createElement('div')
    fixture.id = 'veritas-render-fixture'
    fixture.className = 'vt-preview'
    fixture.style.cssText = 'width:520px;margin:24px auto;max-height:none;overflow:visible'
    document.body.style.overflow = 'auto'
    document.body.append(fixture)
    const root = ReactDOM.createRoot(fixture)
    const avatars = [
      { id: 1001, name: 'Март 7', damage: 125000, overkill: 15000, byType: { Basic: 25000, Ultimate: 100000 }, properties: { CurrentHP: 3000, MaxHP: 3500, ActionDelay: 13.5 }, dead: false },
      { id: 1002, name: 'Дань Хэн', damage: 280000, overkill: 12000, byType: { Skill: 200000, Ultimate: 80000 }, properties: { CurrentHP: 2100, MaxHP: 2800 }, dead: false },
    ]
    const battle = { id: 12345, stageId: 1001, startedAt: Date.now(), endedAt: null, partial: true, interrupted: false, avatars, enemies: [{ uid: 42, id: 1, name: 'Противник с очень длинным именем', properties: { CurrentHP: 25000, MaxHP: 100000, Attack: 1200, Defence: 700, Speed: 125, ActionDelay: 20 }, dead: false }], totalDamage: 405000, actionValue: 231.5, wave: 1, cycle: 2, maxWaves: 3, maxCycles: 10, skills: [], entityTurns: [], turnHistory: [{ actionValue: 100, wave: 1, cycle: 1, avatarsTurnDamage: [25000, 100000], totalDamage: 125000 }, { actionValue: 150, wave: 1, cycle: 1, avatarsTurnDamage: [100000, 80000], totalDamage: 180000 }, { actionValue: 231.5, wave: 1, cycle: 2, avatarsTurnDamage: [0, 100000], totalDamage: 100000 }], avHistory: [] }
    let settings = { ...defaultSettings, views: { legend: true, characters: true, breakdown: true, enemies: true, metrics: true, graph: true, logs: true } }
    const snapshot = { revision: 1, connected: true, active: true, battle, history: [], diagnostics: ['Подключение установлено', 'Неизвестное событие: пример диагностики'] }
    const render = () => root.render(React.createElement(BattleView, { battle, snapshot, settings, patch: (changes) => { settings = { ...settings, ...changes }; render() } }))
    window.__veritasFixture = { theme: (theme) => { settings = { ...settings, theme }; render() } }
    render()
  })
  await page.getByRole('heading', { name: 'Типы урона', exact: true }).waitFor()
  await page.getByRole('button', { name: 'Персонаж', exact: true }).click()
  await page.getByLabel('Тип диаграммы').selectOption('bar')
  assert.equal(await page.getByRole('table').count(), 2)
  await page.getByLabel('Тип диаграммы').selectOption('pie')
  await page.locator('#veritas-render-fixture').screenshot({ path: '../target/ui-evidence/veritas-battle-dark.png' })
  await page.evaluate(() => window.__veritasFixture.theme('light'))
  await page.locator('#veritas-render-fixture').screenshot({ path: '../target/ui-evidence/veritas-battle-light.png' })
  await page.locator('.vt-section--metrics summary').click()
  await page.locator('.vt-section--metrics').getByRole('cell', { name: '245', exact: true }).waitFor()
  await page.locator('.vt-enemies summary').click()
  await page.locator('.vt-enemies .vt-property-list').getByText('251,5', { exact: true }).waitFor()
  await page.setViewportSize({ width: 520, height: 800 })
  assert.equal(await page.locator('#veritas-render-fixture').evaluate((el) => el.scrollWidth <= el.clientWidth), true)
  for (const scale of [.5, 1, 1.5, 2, 2.5]) {
    const bounds = await page.locator('#veritas-render-fixture').evaluate((el, scale) => {
      el.className = 'vt-overlay'
      el.removeAttribute('style')
      el.style.zoom = String(scale)
      el.style.setProperty('--vt-scale', String(scale))
      const rect = el.getBoundingClientRect()
      return { right: rect.right, bottom: rect.bottom, scrollable: el.scrollHeight > el.clientHeight }
    }, scale)
    assert.ok(bounds.right <= 521 && bounds.bottom <= 801, `scaled overlay stays inside viewport at ${scale}`)
    assert.equal(bounds.scrollable, true, `all active views remain scrollable at ${scale}`)
  }
  assert.deepEqual(errors, [])
  console.log('PASS: single Veritas section, tabs, explicit config load, populated dark/light views, category controls, no errors or renderer overflow')
} finally {
  await browser.close()
}
