// Run with a dev server, or NOUS_CDP=http://localhost:9223 for an isolated debug WebView.
import { resolve } from 'node:path'
process.env.PLAYWRIGHT_BROWSERS_PATH ??= resolve('../target/playwright')
const { chromium } = await import('playwright')
import assert from 'node:assert/strict'
import { mkdir } from 'node:fs/promises'

const connected = !!process.env.NOUS_CDP
const browser = connected
  ? await chromium.connectOverCDP(process.env.NOUS_CDP)
  : await chromium.launch({ headless: true })
const errors = []
const context = connected ? browser.contexts()[0] : await browser.newContext({ viewport: { width: 1400, height: 860 } })
const page = connected ? context.pages().find((p) => !p.url().startsWith('devtools:')) : await context.newPage()
assert.ok(page, 'debug WebView must expose one application page')
page.setDefaultTimeout(15_000)
if (connected) {
  const identifier = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('plugin:app|identifier'))
  assert.equal(identifier, 'com.nous.devtest', 'refuse to change a production WebView')
}
page.on('pageerror', (error) => errors.push(String(error)))
page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
const ids = ['overview', 'map', 'characters', 'inventory', 'design-system']
if (!connected) await page.goto(process.env.NOUS_UI_URL ?? 'http://localhost:5173/')
await page.evaluate(() => { location.hash = '/settings' })
if (process.env.NOUS_EXPECT_USER_MODE === 'true') {
  await page.evaluate((ids) => localStorage.setItem('nous.modules.v1', JSON.stringify({ disabled: ids, settings: {} })), ids)
  await page.reload()
  await page.getByRole('heading', { name: 'Настройки', exact: true }).waitFor()
  assert.equal(await page.getByRole('heading', { name: 'Модули', exact: true }).count(), 0)
  for (const id of ids.filter((id) => id !== 'design-system')) assert.equal(await page.locator(`a[href="#/${id}"]`).count(), 1)
  assert.equal(await page.locator('a[href="#/design-system"]').count(), 0)
  assert.deepEqual(errors, [])
  console.log('User mode: no module switches, dev choices ignored, four user tabs enabled')
  await browser.close()
  process.exit(0)
}
await page.getByRole('heading', { name: 'Модули', exact: true }).waitFor()

if (process.env.NOUS_EXPECT_PERSISTED === 'true') {
  for (const id of ids) assert.equal(await page.locator(`a[href="#/${id}"]`).count(), 0)
  console.log('Persisted all-off selection survived application restart')
}
const title = { overview: 'Обзор', map: 'Карта', characters: 'Персонажи', inventory: 'Инвентарь', 'design-system': 'Дизайн-система' }
async function setModule(id, enabled) {
  const checkbox = page.getByRole('checkbox', { name: title[id], exact: true })
  if (await checkbox.isChecked() !== enabled) {
    await page.locator('label.ui-switch').filter({ has: checkbox }).click()
  }
  await page.waitForFunction(({ title, enabled }) => {
    const row = [...document.querySelectorAll('label.ui-switch')].find((el) => el.querySelector('.ui-switch__label')?.textContent === title)
    return row?.querySelector('input')?.checked === enabled && !row.querySelector('input').disabled
  }, { title: title[id], enabled })
  assert.equal(await checkbox.isChecked(), enabled)
}
for (const id of ids) {
  await setModule(id, false)
  await page.locator(`a[href="#/${id}"]`).waitFor({ state: 'detached' })
}
assert.deepEqual(await page.locator('a.nav-item').allTextContents(), ['Настройки'])
assert.equal(await page.locator('.nav-group__title').count(), 0)
await page.getByRole('status').filter({ hasText: 'Все модули отключены' }).waitFor()
await page.reload()
await page.getByRole('heading', { name: 'Модули', exact: true }).waitFor()
for (const id of ids) {
  assert.equal(await page.getByRole('checkbox', { name: title[id], exact: true }).isChecked(), false)
  await page.evaluate((id) => { location.hash = `/${id}/disabled-child` }, id)
  await page.waitForURL('**/#/settings')
}
await page.evaluate(() => { location.hash = '/' })
await page.waitForURL('**/#/settings')

if (connected) {
  const invoke = (command) => page.evaluate((command) => window.__TAURI_INTERNALS__.invoke(command), command)
  await page.waitForFunction(async () => (await window.__TAURI_INTERNALS__.invoke('get_sniffer_status')).state === 'stopped')
  for (const command of ['get_profile', 'get_inventory', 'get_relics', 'get_world', 'get_battles', 'get_gacha', 'get_snapshots', 'get_notes']) {
    const result = await invoke(command)
    assert.equal(typeof result.revision, 'number')
    assert.ok('data' in result)
  }
  console.log('Native IPC: all Versioned reads work; capture status stopped')
}

await mkdir('../target/ui-evidence', { recursive: true })
await page.screenshot({ path: `../target/ui-evidence/${connected ? 'debug' : 'browser'}-all-modules-off.png` })
// Re-enabling removes the empty state and restores exactly that route/menu entry.
await setModule('map', true)
await page.locator('a[href="#/map"]').click()
await page.getByRole('heading', { name: 'Карта', exact: true }).waitFor()
await page.locator('a[href="#/settings"]').click()
await setModule('map', false)
await page.locator('a[href="#/map"]').waitFor({ state: 'detached' })
if (!connected) {
  await page.setViewportSize({ width: 1000, height: 640 })
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true, 'minimum window has no horizontal overflow')
}
const saved = await page.evaluate((connected) => connected
  ? window.__TAURI_INTERNALS__.invoke('get_module_configuration')
  : JSON.parse(localStorage.getItem('nous.modules.v1')), connected)
assert.deepEqual([...saved.disabled].sort(), [...ids].sort(), 'all disabled choices must be persisted after route changes')
assert.deepEqual(errors, [], 'no console errors or uncaught exceptions')
console.log('UI smoke: five modules off, empty categories hidden, redirects, reload persistence, re-enable, zero console errors')
if (connected && process.env.NOUS_RESTART_AFTER === 'true') {
  try { await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('plugin:process|restart')) }
  catch (error) { if (!/closed|destroyed|disconnected/i.test(String(error))) throw error }
  console.log('Requested immediate native restart after the last saved toggle')
}
await browser.close().catch(() => {})
