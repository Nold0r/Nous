import { useEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { openPath } from '@tauri-apps/plugin-opener'
import { Button, Select, Switch, Tabs } from '../ui'
import { useVeritasStatus } from '../lib/veritas'
import { BattleView } from './BattleView'
import { checkUpdate, exportBattle, getBattle, installUpdate, overlayAction, readPluginConfig, useOverlay, writePluginConfig } from './api'
import { columnLabels, formatNumber, themeLabels, viewLabels } from './presentation'
import type { Battle, ColumnKey, OverlaySettings, PluginConfig, PluginOptions, Snapshot, ThemeName, ViewKey } from './types'
import './veritas.css'

type Patch = (changes: Partial<OverlaySettings>) => void

/** Label on the left, control on the right — same rhythm as `Switch`. */
function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="vt-row">
      <span className="vt-row__text">
        <span>{label}</span>
        {hint && <span className="vt-row__hint">{hint}</span>}
      </span>
      {children}
    </div>
  )
}

function Slider({ value, min, max, step = 1, onChange }: { value: number; min: number; max: number; step?: number; onChange: (value: number) => void }) {
  return (
    <span className="vt-slider">
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
      <output>{formatNumber(value)}%</output>
    </span>
  )
}

/** Commits on blur/Enter, so half-typed paths are never saved. */
function TextInput({ value, placeholder, onChange }: { value: string; placeholder?: string; onChange: (value: string) => void }) {
  const [draft, setDraft] = useState(value)
  const input = useRef<HTMLInputElement>(null)
  useEffect(() => { if (document.activeElement !== input.current) setDraft(value) }, [value])
  return <input ref={input} className="vt-input" type="text" value={draft} placeholder={placeholder} onChange={(e) => setDraft(e.target.value)} onBlur={() => { if (draft !== value) onChange(draft) }} onKeyDown={(e) => { if (e.key === 'Enter') e.currentTarget.blur() }} />
}

const anchors = [
  { value: 'top-right', label: 'Справа сверху' },
  { value: 'top-left', label: 'Слева сверху' },
  { value: 'bottom-right', label: 'Справа снизу' },
  { value: 'bottom-left', label: 'Слева снизу' },
] as const

function WindowTab({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  return <>
    <Switch label="Показывать в бою автоматически" hint="Окно появляется при начале боя и скрывается после него." checked={settings.enabled} onChange={(enabled) => patch({ enabled })} />
    <Switch label="Пропускать клики в игру" hint="Окно не перехватывает мышь и фокус." checked={settings.clickThrough} onChange={(clickThrough) => patch({ clickThrough })} />
    <Row label="Положение"><Select title="Положение" value={settings.anchor} options={anchors} onChange={(anchor) => patch({ anchor })} /></Row>
    <Row label="Масштаб"><Slider value={Math.round(settings.scale * 100)} min={50} max={250} step={5} onChange={(v) => patch({ scale: v / 100 })} /></Row>
    <Row label="Непрозрачность"><Slider value={Math.round(settings.opacity * 100)} min={10} max={100} onChange={(v) => patch({ opacity: v / 100 })} /></Row>
  </>
}

function ViewTab({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  const toggleColumn = (key: ColumnKey) =>
    patch({ columns: settings.columns.includes(key) ? settings.columns.filter((c) => c !== key) : [...settings.columns, key] })
  return <>
    {(Object.entries(viewLabels) as [ViewKey, string][]).map(([key, label]) => (
      <Switch key={key} label={label} checked={settings.views[key]} onChange={(on) => patch({ views: { ...settings.views, [key]: on } })} />
    ))}
    <Row label="Колонки таблицы">
      <span className="vt-chips">
        {(Object.entries(columnLabels) as [ColumnKey, string][]).map(([key, label]) => (
          <Button key={key} size="sm" variant={settings.columns.includes(key) ? 'primary' : 'secondary'} aria-pressed={settings.columns.includes(key)} onClick={() => toggleColumn(key)}>{label}</Button>
        ))}
      </span>
    </Row>
    <Row label="Сортировка"><Select title="Сортировка" value={settings.metric} options={[{ value: 'damage', label: 'Общий урон' }, { value: 'dpav', label: 'Урон / AV' }] as const} onChange={(metric) => patch({ metric })} /></Row>
    <Row label="Акцентный цвет"><Select title="Акцентный цвет" value={settings.theme} options={(Object.entries(themeLabels) as [ThemeName, string][]).map(([value, label]) => ({ value, label }))} onChange={(theme) => patch({ theme })} /></Row>
  </>
}

function ExportTab({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  return <>
    <Switch label="Сохранять завершённые бои" hint="JSON и CSV после каждого боя." checked={settings.autoExport} onChange={(autoExport) => patch({ autoExport })} />
    <Switch label="Раскладывать по датам" checked={settings.dateFolders} onChange={(dateFolders) => patch({ dateFolders })} />
    <Row label="Папка" hint="Пусто — папка Nous по умолчанию.">
      <TextInput value={settings.exportDirectory ?? ''} placeholder="По умолчанию" onChange={(v) => patch({ exportDirectory: v.trim() || null })} />
    </Row>
  </>
}

const locales = { ru: 'Русский', en: 'English', ja: '日本語', zh: '中文', de: 'Deutsch', fr: 'Français', es: 'Español', it: 'Italiano', pt: 'Português', nl: 'Nederlands', vi: 'Tiếng Việt', pl: 'Polski' }

/** Files the plugin itself reads at game start. Loaded only on demand. */
function PluginSettings() {
  const status = useVeritasStatus()
  const [config, setConfig] = useState<PluginConfig | null>(null)
  const [update, setUpdate] = useState<Awaited<ReturnType<typeof checkUpdate>> | null>(null)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null)
  const run = (action: () => Promise<unknown>, ok?: string) => {
    setBusy(true); setNote(null)
    action().then(() => { if (ok) setNote({ ok: true, text: ok }) }).catch((e) => setNote({ ok: false, text: String(e) })).finally(() => setBusy(false))
  }
  const patch = (changes: Partial<PluginOptions>) => { if (config) setConfig({ ...config, options: { ...config.options, ...changes } }) }
  const options = config?.options
  const outdated = update != null && status?.version !== update.version
  return <div className="vt-plugin">
    <p className="vt-caption">Плагин читает эти настройки при запуске игры: сохраняйте их при закрытой игре.</p>
    {!options && <Button size="sm" disabled={busy || !isTauri()} onClick={() => run(async () => setConfig(await readPluginConfig()))}>Загрузить настройки плагина</Button>}
    {options && <fieldset className="vt-plugin__fields" disabled={busy}>
      <Switch label="Скрывать встроенные окна Veritas" hint="Урон показывает окно Nous. Стартовые уведомления плагина скрыть нельзя." checked={options.suppressNative} onChange={(suppressNative) => patch({ suppressNative })} />
      <Row label="Язык плагина"><Select title="Язык плагина" value={options.locale} options={Object.entries(locales).map(([value, label]) => ({ value, label }))} onChange={(locale) => patch({ locale })} /></Row>
      <div className="vt-actions"><Button size="sm" variant="primary" onClick={() => run(async () => setConfig(await writePluginConfig(options)), 'Сохранено. Применится при следующем запуске игры.')}>Сохранить</Button></div>
    </fieldset>}
    <Row label="Версия плагина" hint={status?.version ? `Установлена ${status.version}` : undefined}>
      {outdated
        ? <Button size="sm" variant="primary" disabled={busy || !status?.enabled} onClick={() => run(installUpdate, `Veritas ${update.version} установлен.`)}>Обновить до {update.version}</Button>
        : <Button size="sm" disabled={busy || !isTauri() || update != null} onClick={() => run(async () => setUpdate(await checkUpdate()))}>{update ? 'Последняя версия' : 'Проверить обновление'}</Button>}
    </Row>
    {outdated && <p className="vt-caption">Поддерживаемые версии игры: {update.gameRange || 'не указаны'}. Игра должна быть закрыта.</p>}
    {note && <p className={note.ok ? 'vt-success' : 'vt-error'} role="status">{note.text}</p>}
  </div>
}

function LastBattle({ snapshot, settings, patch }: { snapshot: Snapshot; settings: OverlaySettings; patch: Patch }) {
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [selected, setSelected] = useState<Battle | null>(null)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ ok: boolean; text: string } | null>(null)
  const [folder, setFolder] = useState<string | null>(null)
  const request = useRef(0)
  const battle = selectedId === null ? snapshot.battle : selected
  const select = (id: number | null) => {
    const seq = ++request.current
    setSelectedId(id); setSelected(null); setNote(null)
    if (id !== null) getBattle(id).then((b) => { if (seq === request.current) setSelected(b) }).catch((e) => setNote({ ok: false, text: String(e) }))
  }
  const save = () => {
    if (!battle) return
    setBusy(true); setNote(null)
    exportBattle(battle.id, 'both')
      .then((r) => { setFolder(r.directory); setNote({ ok: true, text: `Сохранено: ${r.files.join(', ')}` }) })
      .catch((e) => setNote({ ok: false, text: String(e) }))
      .finally(() => setBusy(false))
  }
  const options = [
    { value: -1, label: snapshot.active ? 'Текущий бой' : 'Последний бой' },
    ...snapshot.history.map((b) => ({ value: b.id, label: `Бой ${b.id} · ${formatNumber(b.totalDamage)} урона${b.partial ? ' · неполный' : ''}` })),
  ]
  return <div className="vt-last">
    <div className="vt-actions">
      <Select title="Бой" value={selectedId ?? -1} options={options} onChange={(id) => select(id === -1 ? null : id)} />
      <Button size="sm" disabled={!isTauri() || !battle || busy} onClick={save}>Сохранить JSON + CSV</Button>
      {folder && <Button size="sm" variant="ghost" onClick={() => void openPath(folder)}>Открыть папку</Button>}
    </div>
    {note && <p className={note.ok ? 'vt-success' : 'vt-error'} role="status">{note.text}</p>}
    <div className="vt-preview"><BattleView battle={battle} snapshot={snapshot} settings={settings} patch={patch} /></div>
  </div>
}

const tabs = [
  { id: 'window', label: 'Окно урона' },
  { id: 'view', label: 'Вид' },
  { id: 'export', label: 'Экспорт' },
] as const
type Tab = (typeof tabs)[number]['id']

/**
 * Overlay settings inside the Veritas section. Shown once the plugin is on,
 * in the debug replay, and in the plain-browser preview.
 */
export function VeritasOverlaySettings({ pluginEnabled }: { pluginEnabled: boolean }) {
  const { state, patch, error } = useOverlay()
  const [tab, setTab] = useState<Tab>('window')
  if (!state) return error ? <p className="vt-error" role="alert">{error}</p> : null
  if (!pluginEnabled && !state.replay && isTauri()) return null
  const { settings, presentation, snapshot } = state
  const status = state.replay
    ? 'Тестовый режим: бои приходят от npm run dev:veritas.'
    : presentation.exclusiveFullscreen
      ? 'Игра в эксклюзивном полноэкранном режиме — окно урона поверх неё не видно. Включите оконный или безрамочный режим.'
      : presentation.message
  return <div className="vt-settings">
    {status && <p className="vt-caption" role="status">{status}</p>}
    {error && <p className="vt-error" role="alert">{error}</p>}
    <Tabs items={tabs} value={tab} onChange={setTab} />
    <div className="vt-tab">
      {tab === 'window' && <WindowTab settings={settings} patch={patch} />}
      {tab === 'view' && <ViewTab settings={settings} patch={patch} />}
      {tab === 'export' && <ExportTab settings={settings} patch={patch} />}
    </div>
    <details className="vt-more">
      <summary>Предпросмотр последнего боя{snapshot.battle ? '' : ' — боёв пока нет'}</summary>
      <LastBattle snapshot={snapshot} settings={settings} patch={patch} />
    </details>
    <details className="vt-more">
      <summary>Дополнительно (настройки плагина)</summary>
      <PluginSettings />
      <Button size="sm" variant="ghost" disabled={!isTauri()} onClick={() => void overlayAction('reset').catch(console.error)}>Сбросить настройки окна урона</Button>
    </details>
  </div>
}
