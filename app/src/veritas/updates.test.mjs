// Browser-only IPC double: update checks and install buttons never reach a native host.
import assert from 'node:assert/strict'
import { resolve } from 'node:path'

process.env.PLAYWRIGHT_BROWSERS_PATH ??= resolve('../target/playwright')
const { chromium } = await import('playwright')
const browser = await chromium.launch({ headless: true })
try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 1000 } })
  await page.goto(`${process.env.NOUS_UI_URL ?? 'http://127.0.0.1:5174/'}#/veritas-overlay`)
  await page.evaluate(async () => {
    const { default: React } = await import('/node_modules/.vite/deps/react.js')
    const { default: ReactDOM } = await import('/node_modules/.vite/deps/react-dom_client.js')
    const { VeritasControls } = await import('/src/veritas/VeritasControls.tsx')
    const { defaultSettings } = await import('/src/veritas/types.ts')
    const callbacks = new Map()
    const events = new Map()
    let callbackId = 0
    let status = { enabled: false, version: null, gameRange: null, gameVersion: null, compatible: null, connected: false }
    const calls = []
    const state = { settings: defaultSettings, snapshot: { revision: 0, connected: false, active: false, battle: null, history: [], diagnostics: [] }, presentation: { targetFound: false, targetVisible: false, exclusiveFullscreen: false, visible: false, message: null }, replay: false, hidden: false }
    window.__TAURI_INTERNALS__ = {
      transformCallback: (callback) => { callbacks.set(++callbackId, callback); return callbackId },
      invoke: async (command, args) => {
        calls.push({ command, args })
        if (command === 'plugin:event|listen') { events.set(args.handler, args.event); return args.handler }
        if (command === 'plugin:event|unlisten') return
        if (command === 'get_veritas_overlay') return state
        if (command === 'get_veritas_status') return status
        if (command === 'check_veritas_update') return { version: args.channel === 'beta' ? '0.2.58-beta.1' : '0.2.57', notes: 'Изменения тестового выпуска', gameRange: '4.3.54 - 4.6.52', prerelease: args.channel === 'beta' }
        if (command === 'enable_veritas') return status
        throw new Error(`Unexpected mocked command: ${command}`)
      },
    }
    window.isTauri = true
    window.__veritasUpdateMock = { calls, consent: () => {
      status = { ...status, enabled: true, version: '0.2.56' }
      for (const [handler, event] of events) if (event === 'veritas-status') callbacks.get(handler)?.({ event, id: handler, payload: status })
    } }
    document.getElementById('root').style.display = 'none'
    const target = document.createElement('div')
    target.style.cssText = 'padding:24px;max-width:1000px;margin:auto'
    document.body.style.overflow = 'auto'
    document.body.append(target)
    ReactDOM.createRoot(target).render(React.createElement(VeritasControls))
  })
  await page.getByRole('tab', { name: 'Плагин', exact: true }).click()
  await page.getByRole('combobox', { name: 'Канал обновления Nous' }).selectOption('beta')
  await page.getByRole('button', { name: 'Проверить обновление', exact: true }).click()
  const install = page.getByRole('button', { name: 'Установить Veritas 0.2.58-beta.1', exact: true })
  await install.waitFor()
  assert.equal(await install.isDisabled(), true, 'update cannot become an alternative initial consent path')
  await page.evaluate(() => window.__veritasUpdateMock.consent())
  await page.waitForFunction(() => [...document.querySelectorAll('button')].some((button) => button.textContent === 'Установить Veritas 0.2.58-beta.1' && !button.disabled))
  await install.click()
  await page.getByText('Veritas обновлён. Новый выпуск загрузится после следующего запуска игры.', { exact: true }).waitFor()
  const installCall = await page.evaluate(() => window.__veritasUpdateMock.calls.find((call) => call.command === 'enable_veritas'))
  assert.deepEqual(installCall.args, { acceptRisk: true, channel: 'beta', updateOnly: true })
  await page.getByRole('combobox', { name: 'Канал обновления Nous' }).selectOption('stable')
  assert.equal(await page.getByRole('button', { name: 'Установить Veritas 0.2.58-beta.1', exact: true }).count(), 0, 'channel switch invalidates checked release')
  await page.getByRole('button', { name: 'Проверить обновление', exact: true }).click()
  await page.getByRole('button', { name: 'Установить Veritas 0.2.57', exact: true }).waitFor()
  assert.deepEqual(await page.evaluate(() => window.__veritasUpdateMock.calls.filter((call) => call.command === 'check_veritas_update').map((call) => call.args)), [{ channel: 'beta' }, { channel: 'stable' }])
  console.log('PASS: explicit stable/beta checks, prior-consent guard, channel invalidation, updateOnly IPC; no native host used')
} finally {
  await browser.close()
}
