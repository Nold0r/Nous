import { useEffect, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { getAutostart, setAutostart } from '../../lib/game'
import { getVersion } from '@tauri-apps/api/app'
import { openPatchNotes } from '../../lib/patch-notes'
import { Button, Page, Panel, StatList, StatRow, Switch } from '../../ui'
import './settings.css'
import { enabledSections, modules } from '../registry'
import { useModules } from '../module-context'

export function SettingsPage() {
  const { enabled, setEnabled, saving, error: moduleError } = useModules()
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
      {modules.length > 0 && (
        <Panel title="Модули">
          <p className="modules-description">Всё, кроме аккаунта, — модули: вкладки в меню и разделы здесь, в Настройках. Изменения применяются сразу и сохраняются.</p>
          {moduleError && <p role="alert">{moduleError}</p>}
          {modules.map((module) => (
            <Switch key={module.id} disabled={saving} checked={enabled.has(module.id)} onChange={(value) => setEnabled(module.id, value)}
              label={module.title} hint={module.kind === 'settings' ? 'Раздел в Настройках' : 'Вкладка в меню'} />
          ))}
          {enabled.size === 0 && <p className="modules-empty" role="status">Все модули отключены. Чтобы вернуть вкладку или раздел, включите его здесь.</p>}
        </Panel>
      )}
      {enabledSections(enabled).map(({ id, section: Section }) => Section && <Section key={id} />)}
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
