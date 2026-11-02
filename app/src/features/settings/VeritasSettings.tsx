import { useState } from 'react'
import { openUrl } from '@tauri-apps/plugin-opener'
import { disableVeritas, enableVeritas, useVeritasStatus } from '../../lib/veritas'
import { Button, Panel, Switch } from '../../ui'
import { VeritasControls } from '../../veritas/VeritasControls'

/**
 * Opt-in for Veritas. Turning it on asks for an explicit acceptance of the
 * risk first: unlike the rest of Nous, it injects into the game.
 */
export function VeritasSettings() {
  const status = useVeritasStatus()
  const [confirming, setConfirming] = useState(false)
  const [accepted, setAccepted] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const run = (action: () => Promise<unknown>) => {
    setBusy(true)
    setError(null)
    action()
      .then(() => {
        setConfirming(false)
        setAccepted(false)
      })
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(false))
  }

  const toggle = (on: boolean) => {
    if (on) setConfirming(true)
    else run(disableVeritas)
  }

  const hint = status?.enabled
    ? [
        `Veritas ${status.version ?? ''} установлен в папку игры.`,
        status.connected ? 'Подключён: урон идёт в Nous.' : 'Подключится, когда игра будет запущена.',
        status.compatible === false &&
          `Версия игры ${status.gameVersion} вне поддерживаемых плагином (${status.gameRange}): игра может падать — выключите опцию.`,
      ]
        .filter(Boolean)
        .join(' ')
    : 'Выключено. В папке игры ничего нет.'

  return (
    <>
    <Panel title="Урон в реальном времени (Veritas)">
      <Switch
        checked={status?.enabled ?? false}
        onChange={toggle}
        disabled={status == null || busy || confirming}
        label="Встроить Veritas в игру"
        hint={error ?? hint}
      />

      {confirming && (
        <div className="risk">
          <p className="risk__title">Это внедрение в игру</p>
          <p>
            <a
              href="https://github.com/hessiser/veritas"
              onClick={(e) => {
                e.preventDefault()
                openUrl('https://github.com/hessiser/veritas').catch(console.error)
              }}
            >
              Veritas
            </a>{' '}
            — сторонний плагин. Nous положит его библиотеку в папку игры, игра загрузит её в свой процесс, и плагин будет
            читать данные боя из памяти игры. Так Nous получает урон каждого удара вживую.
          </p>
          <p>
            Это <b>нарушает Пользовательское соглашение HoYoverse</b>. Античит может это обнаружить, вплоть до{' '}
            <b>блокировки аккаунта</b>. Автор Veritas не гарантирует защиту от санкций. Разработчики Nous{' '}
            <b>не несут ответственности</b> за последствия: включая опцию, вы действуете на свой страх и риск.
          </p>
          <p>
            Закройте игру перед включением. После обновления игры плагин может перестать работать или ронять игру, пока не
            выйдет его новая версия. Выключение опции удаляет файл из папки игры.
          </p>
          <label className="risk__accept">
            <input type="checkbox" checked={accepted} onChange={(e) => setAccepted(e.target.checked)} />
            Я понимаю риски, принимаю их и снимаю ответственность с разработчиков Nous
          </label>
          <div className="risk__actions">
            <Button variant="ghost" onClick={() => setConfirming(false)} disabled={busy}>
              Отмена
            </Button>
            <Button variant="primary" onClick={() => run(() => enableVeritas(true))} disabled={!accepted || busy}>
              {busy ? 'Установка…' : 'Включить на свой риск'}
            </Button>
          </div>
        </div>
      )}
    </Panel>
    <VeritasControls />
    </>
  )
}
