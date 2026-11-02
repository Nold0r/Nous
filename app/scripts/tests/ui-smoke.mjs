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
page.on('pageerror', (error) => errors.push(String(error)))
page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()) })
const ids = ['overview', 'map', 'characters', 'relics', 'design-system']
if (!connected) await page.goto(process.env.NOUS_UI_URL ?? 'http://localhost:5173/')
await page.evaluate(() => { location.hash = '/settings' })
await page.getByRole('heading', { name: 'Модули', exact: true }).waitFor()

if (process.env.NOUS_EXPECT_PERSISTED === 'true') {
  for (const id of ids) assert.equal(await page.locator(`a[href="#/${id}"]`).count(), 0)
  console.log('Persisted all-off selection survived application restart')
}
const title = { overview: 'Обзор', map: 'Карта', characters: 'Персонажи', relics: 'Реликвии', 'design-system': 'Дизайн-система' }
async function setModule(id, enabled) {
  const checkbox = page.getByRole('checkbox', { name: title[id], exact: true })
  if (await checkbox.isChecked() !== enabled) {
    await page.locator('label.ui-switch').filter({ has: checkbox }).click()
  }
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
  for (const command of ['get_profile', 'get_inventory', 'get_relics', 'get_world', 'get_battles', 'get_gacha', 'get_snapshots']) {
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
assert.deepEqual(errors, [], 'no console errors or uncaught exceptions')
console.log('UI smoke: five modules off, empty categories hidden, redirects, reload persistence, re-enable, zero console errors')
await browser.close()
