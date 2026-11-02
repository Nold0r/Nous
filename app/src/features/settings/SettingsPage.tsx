import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getAutostart, setAutostart } from '../../lib/game'
import { getVersion } from '@tauri-apps/api/app'
import { openPatchNotes } from '../../lib/patch-notes'
import { Button, Page, Panel, StatList, StatRow, Switch } from '../../ui'
import { VeritasSettings } from './VeritasSettings'
import './settings.css'

export function SettingsPage() {
  const [autostart, setAutostartState] = useState<boolean | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [version, setVersion] = useState<string | null>(null)

  useEffect(() => {
    if (!isTauri()) return
    getAutostart().then(setAutostartState).catch(console.error)
    getVersion().then(setVersion).catch(console.error)
  }, [])

  const toggle = (enabled: boolean) => {
    setBusy(true)
    setError(null)
    setAutostart(enabled)
      .then(setAutostartState)
      .catch(() => setError('Не получилось. Подтвердите запрос прав администратора, чтобы изменить настройку.'))
      .finally(() => setBusy(false))
  }

  return (
    <Page title="Настройки">
      <Panel title="Запуск">
        <Switch
          checked={autostart ?? false}
          onChange={toggle}
          disabled={autostart == null || busy}
          label="Запускаться вместе с игрой"
          hint={
            error ??
            'Nous тихо стартует с Windows в трее с правами администратора и открывается, когда запускается Honkai: Star Rail. Так вход в игру всегда будет пойман. При включении Windows один раз спросит разрешение.'
          }
        />
      </Panel>
      <VeritasSettings />
      <Panel
        title="О программе"
        aside={
          <Button size="sm" onClick={() => openPatchNotes()}>
            Что нового
          </Button>
        }
      >
        <StatList>
          <StatRow label="Версия Nous" value={version ?? '—'} />
        </StatList>
      </Panel>
    </Page>
  )
}
