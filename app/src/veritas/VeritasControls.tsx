import { useEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { openPath } from '@tauri-apps/plugin-opener'
import { Download, Eye, EyeOff, RefreshCw, Settings2 } from 'lucide-react'
import { Button, Panel, Switch } from '../ui'
import { useVeritasStatus } from '../lib/veritas'
import { BattleView } from './BattleView'
import { checkUpdate, exportBattle, getBattle, installCheckedUpdate, overlayAction, readPluginConfig, useOverlay, writePluginConfig } from './api'
import type { UpdateChannel } from './api'
import { columnLabels, formatNumber, pluginThemes, themeLabels, viewLabels } from './presentation'
import type { Battle, ColumnKey, OverlaySettings, PluginConfig, PluginOptions, ThemeName, ViewKey } from './types'
import './veritas.css'

type Patch = (changes: Partial<OverlaySettings>) => void

function Field({ label, children, hint }: { label: string; children: ReactNode; hint?: string }) {
  return <label className="vt-field"><span>{label}</span>{children}{hint && <small>{hint}</small>}</label>
}

function Numeric({ label, value, min, max, step = 1, change, unit = '' }: { label: string; value: number; min: number; max: number; step?: number; change: (value: number) => void; unit?: string }) {
  return <Field label={label}><span className="vt-range"><input type="range" min={min} max={max} step={step} value={value} onChange={(e) => change(Number(e.target.value))} /><output>{formatNumber(value)}{unit}</output></span></Field>
}

function NumericInput({ label, value, min, max, change }: { label: string; value: number; min: number; max: number; change: (value: number) => void }) {
  return <Field label={label}><input type="number" value={value} min={min} max={max} step="1" onChange={(e) => { const number = e.target.valueAsNumber; if (Number.isFinite(number)) change(Math.max(min, Math.min(max, number))) }} /></Field>
}

/** Commit text only after editing, so intermediate hotkeys/paths are never persisted. */
function TextSetting({ label, value, change, hint, placeholder }: { label: string; value: string; change: (value: string) => void; hint?: string; placeholder?: string }) {
  const [draft, setDraft] = useState(value)
  const input = useRef<HTMLInputElement>(null)
  useEffect(() => { if (document.activeElement !== input.current) setDraft(value) }, [value])
  return <Field label={label} hint={hint}><input ref={input} type="text" value={draft} placeholder={placeholder} onChange={(e) => setDraft(e.target.value)} onBlur={() => { if (draft !== value) change(draft) }} onKeyDown={(e) => { if (e.key === 'Enter') e.currentTarget.blur() }} /></Field>
}

const tabs = ['Представления', 'Оформление', 'Окно и клавиши', 'Экспорт', 'Плагин', 'Справка'] as const
type Tab = typeof tabs[number]

function ViewSettings({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  return <>
    <div className="vt-choice-grid">{(Object.entries(viewLabels) as [ViewKey, string][]).map(([key, label]) => <Switch key={key} label={label} checked={settings.views[key]} onChange={(value) => patch({ views: { ...settings.views, [key]: value } })} />)}</div>
    <h4>Колонки таблицы персонажей</h4>
    <div className="vt-checks">{(Object.entries(columnLabels) as [ColumnKey, string][]).map(([key, label]) => <label key={key}><input type="checkbox" checked={settings.columns.includes(key)} onChange={(e) => patch({ columns: e.target.checked ? [...settings.columns, key] : settings.columns.filter((column) => column !== key) })} />{label}</label>)}</div>
    <div className="vt-form-grid"><Field label="Показатель и сортировка"><select value={settings.metric} onChange={(e) => patch({ metric: e.target.value as OverlaySettings['metric'] })}><option value="damage">Общий урон</option><option value="dpav">Урон / AV</option></select></Field><Field label="Подписи в таблице"><select value={settings.legendSize} onChange={(e) => patch({ legendSize: e.target.value as OverlaySettings['legendSize'] })}><option value="small">Маленькие</option><option value="body">Обычные</option><option value="large">Крупные</option></select></Field></div>
    <h4>Распределение по типам урона</h4>
    <div className="vt-choice-grid"><Switch label="Легенда диаграммы" checked={settings.showLegend} onChange={(showLegend) => patch({ showLegend })} /><Switch label="Подробная таблица" checked={settings.showBreakdownTable} onChange={(showBreakdownTable) => patch({ showBreakdownTable })} /></div>
    <p className="vt-caption">Область, персонаж, тип диаграммы и ось графика переключаются в самом предпросмотре. Настройки сохраняются сразу.</p>
  </>
}

function Appearance({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  const slotLabels = ['Фон', 'Панель', 'Выделенная панель', 'Наведение', 'Граница', 'Сильная граница', 'Вторичный текст', 'Основной текст', 'Акцент', 'Выбор', 'Информация', 'Успех']
  const initialColors = ['#100b10', '#201b20', '#2a252a', '#302b30', '#454047', '#625b63', '#b6afa9', '#ecebe6', '#d8b875', '#e9e4da', '#8ec5ff', '#8fd4a4']
  return <>
    <Field label="Тема Nous"><select value={settings.theme} onChange={(e) => patch({ theme: e.target.value as ThemeName })}>{Object.entries(themeLabels).map(([key, label]) => <option key={key} value={key}>{label}</option>)}</select></Field>
    <Numeric label="Прозрачность окна" value={Math.round(settings.opacity * 100)} min={10} max={100} change={(value) => patch({ opacity: value / 100 })} unit="%" />
    <Numeric label="Насыщенность круговой диаграммы" value={Math.round(settings.pieOpacity * 100)} min={0} max={100} change={(value) => patch({ pieOpacity: value / 100 })} unit="%" />
    <Switch label="Собственные цвета" checked={settings.useCustomColors} onChange={(useCustomColors) => patch({ useCustomColors, customColors: settings.customColors.length === 12 ? settings.customColors : initialColors })} />
    {settings.useCustomColors && <div className="vt-color-grid">{slotLabels.map((label, index) => <label key={label}><input aria-label={label} type="color" value={settings.customColors[index] ?? initialColors[index]} onChange={(e) => { const colors = [...initialColors]; settings.customColors.forEach((color, i) => { colors[i] = color }); colors[index] = e.target.value; patch({ customColors: colors }) }} /><span>{label}</span></label>)}</div>}
    <h4>Размеры шрифта</h4>
    <div className="vt-form-grid">{Object.entries({ body: 'Основной текст', heading: 'Заголовки', small: 'Подписи', legend: 'Таблица' }).map(([key, label]) => <Numeric key={key} label={label} value={settings.fontSizes[key] ?? 14} min={8} max={48} change={(value) => patch({ fontSizes: { ...settings.fontSizes, [key]: value } })} />)}</div>
    <Switch label="Сообщение для трансляции" checked={settings.streamerMode} onChange={(streamerMode) => patch({ streamerMode })} hint="Добавляет подпись внизу оверлея Nous." />
    <TextSetting label="Текст подписи" value={settings.streamerMessage} change={(streamerMessage) => patch({ streamerMessage })} />
  </>
}

function WindowSettings({ settings, patch, run }: { settings: OverlaySettings; patch: Patch; run: (action: () => Promise<unknown>, message?: string) => void }) {
  const hotkeyLabels: Record<string, string> = { menu: 'Открыть настройки', hide: 'Показать / скрыть', zoomIn: 'Увеличить масштаб', zoomOut: 'Уменьшить масштаб', zoomReset: 'Масштаб 100%' }
  return <>
    <Switch label="Оверлей Nous включён" checked={settings.enabled} onChange={(enabled) => patch({ enabled })} hint="Показывается поверх окна игры. Установка плагина управляется отдельно выше." />
    <Switch label="Пропускать нажатия в игру" checked={settings.clickThrough} onChange={(clickThrough) => patch({ clickThrough })} hint="По умолчанию оверлей не забирает мышь и фокус. Всеми представлениями можно управлять в предпросмотре." />
    <Numeric label="Масштаб" value={Math.round(settings.scale * 100)} min={50} max={250} step={5} change={(value) => patch({ scale: value / 100 })} unit="%" />
    <div className="vt-form-grid"><Field label="Угол окна игры"><select value={settings.anchor} onChange={(e) => patch({ anchor: e.target.value as OverlaySettings['anchor'] })}><option value="top-right">Справа сверху</option><option value="top-left">Слева сверху</option><option value="bottom-right">Справа снизу</option><option value="bottom-left">Слева снизу</option></select></Field><NumericInput label="Ширина, px" value={settings.width} min={300} max={1400} change={(width) => patch({ width })} /><NumericInput label="Высота, px" value={settings.height} min={180} max={1400} change={(height) => patch({ height })} /><NumericInput label="Отступ по горизонтали, px" value={settings.offsetX} min={0} max={4000} change={(offsetX) => patch({ offsetX })} /><NumericInput label="Отступ по вертикали, px" value={settings.offsetY} min={0} max={4000} change={(offsetY) => patch({ offsetY })} /></div>
    <h4>Глобальные горячие клавиши</h4>
    <p className="vt-caption">Например, Ctrl+Shift+M. Изменение применяется после завершения ввода. Системные конфликты будут показаны как ошибка.</p>
    <div className="vt-form-grid">{Object.entries(hotkeyLabels).map(([key, label]) => <TextSetting key={key} label={label} value={settings.hotkeys[key] ?? ''} change={(value) => patch({ hotkeys: { ...settings.hotkeys, [key]: value } })} />)}</div>
    <p className="vt-notice">Ctrl+M и Ctrl+H остаются встроенными клавишами Veritas. Официальный плагин не позволяет их отключить. Выберите другие сочетания для Nous.</p>
    <Button size="sm" icon={RefreshCw} onClick={() => run(() => overlayAction('reset'), 'Настройки оверлея сброшены. История боёв сохранена.')}>Сбросить настройки оверлея</Button>
  </>
}

function ExportSettings({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  return <>
    <Switch label="Автоматически сохранять завершённые бои" checked={settings.autoExport} onChange={(autoExport) => patch({ autoExport })} hint="Nous сохранит JSON и CSV после получения события завершения боя." />
    <Switch label="Папки по датам" checked={settings.dateFolders} onChange={(dateFolders) => patch({ dateFolders })} />
    <TextSetting label="Папка экспорта Nous" value={settings.exportDirectory ?? ''} change={(value) => patch({ exportDirectory: value.trim() || null })} placeholder="Папка Nous по умолчанию" hint="Абсолютный путь. Пустое поле возвращает папку по умолчанию. Путь проверяется при сохранении." />
    <p className="vt-caption">JSON использует структуру Veritas для анализа боя. CSV содержит строки персонажей и отдельных навыков. Ручной экспорт выбранного боя доступен под предпросмотром.</p>
    <p className="vt-notice">Сохранение Nous и автоэкспорт самого Veritas — отдельные функции. Настройки этого раздела не изменяют файлы плагина.</p>
  </>
}

function PluginSettings() {
  const status = useVeritasStatus()
  const [config, setConfig] = useState<PluginConfig | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [update, setUpdate] = useState<Awaited<ReturnType<typeof checkUpdate>> | null>(null)
  const [channel, setChannel] = useState<UpdateChannel>('stable')
  const patch = (changes: Partial<PluginOptions>) => { if (config) { setConfig({ ...config, options: { ...config.options, ...changes } }); setMessage(null) } }
  const load = async () => {
    setBusy(true); setError(null); setMessage(null)
    try { setConfig(await readPluginConfig()) } catch (e) { setError(`Не удалось прочитать настройки: ${String(e)}`) } finally { setBusy(false) }
  }
  const save = async () => {
    if (!config) return
    setBusy(true); setError(null); setMessage(null)
    try { setConfig(await writePluginConfig(config.options)); setMessage('Настройки плагина сохранены. Они вступят в силу после следующего запуска игры.') } catch (e) { setError(`Настройки не записаны: ${String(e)}`) } finally { setBusy(false) }
  }
  const check = async () => {
    setBusy(true); setError(null); setUpdate(null); setMessage(null)
    try { setUpdate(await checkUpdate(channel)) } catch (e) { setError(`Проверка обновления не удалась: ${String(e)}`) } finally { setBusy(false) }
  }
  const install = async () => {
    if (!update || !status?.enabled) return
    setBusy(true); setError(null); setMessage(null)
    try { await installCheckedUpdate(channel); setMessage(`Veritas обновлён. Новый выпуск загрузится после следующего запуска игры.`) } catch (e) { setError(`Обновление не установлено: ${String(e)}`) } finally { setBusy(false) }
  }
  const options = config?.options
  return <>
    <p className="vt-notice">Закройте игру перед записью настроек Veritas. Плагин читает файлы при запуске: после записи требуется новый запуск игры. Официальная DLL остаётся неизменённой.</p>
    <div className="vt-actions"><Button size="sm" disabled={busy || !isTauri()} onClick={() => void load()}>{busy ? 'Обработка…' : config ? 'Перечитать файлы' : 'Прочитать настройки плагина'}</Button></div>
    <h4>Официальные выпуски Veritas</h4>
    <Field label="Канал обновления Nous"><select value={channel} disabled={busy} onChange={(e) => { setChannel(e.target.value as UpdateChannel); setUpdate(null); setMessage(null) }}><option value="stable">Стабильный</option><option value="beta">Бета, включая предварительные выпуски</option></select></Field>
    <p className="vt-caption">{status?.enabled ? `Установлен Veritas ${status.version || 'неизвестной версии'}. ` : 'Обновление доступно после включения Veritas и принятия риска выше. '}Выбор канала не меняет настройки собственного обновлятора Veritas.</p>
    {channel === 'beta' && <p className="vt-notice">Бета-выпуск может предназначаться для тестового клиента игры и быть несовместимым с обычным клиентом. Перед установкой сверьте поддерживаемые версии игры.</p>}
    <Button size="sm" disabled={busy || !isTauri()} onClick={() => void check()}>Проверить обновление</Button>
    {error && <p className="vt-error" role="alert">{error}</p>}{message && <p className="vt-success" role="status">{message}</p>}
    {update && <div className="vt-update"><h4>Veritas {update.version}{update.prerelease ? ' · Предварительный выпуск' : ' · Стабильный выпуск'}</h4><p className="vt-caption">Поддерживаемая игра: {update.gameRange || 'не указана автором'}. Официальный издатель и SHA-256 проверяются при установке.</p><details className="vt-details"><summary>Изменения версии</summary><p className="vt-release-notes">{update.notes || 'Описание отсутствует.'}</p></details><Button size="sm" variant="primary" disabled={busy || !status?.enabled || status.version === update.version} onClick={() => void install()}>{status?.version === update.version ? 'Этот выпуск уже установлен' : `Установить Veritas ${update.version}`}</Button><p className="vt-caption">Игра должна быть закрыта. Обновляется только файл, ранее установленный Nous; принятие риска и проверки издателя / SHA-256 сохраняются.</p></div>}
    {config && options && <fieldset className="vt-plugin-fields" disabled={busy}>
      <p className="vt-path">{config.directory}</p>
      {config.warning && <p className="vt-notice">{config.warning}</p>}
      <Switch label="Подавлять встроенные окна Veritas" checked={options.suppressNative} onChange={(suppressNative) => patch({ suppressNative })} hint="Закрывает окна, убирает подпись и отключает автоматический показ через штатные JSON/RON-настройки." />
      <p className="vt-caption">Полное скрытие egui невозможно: стартовые уведомления, ошибки и встроенные клавиши могут остаться. Это ограничение официальной DLL.</p>
      <div className="vt-form-grid"><Field label="Язык встроенного UI"><select value={options.locale} onChange={(e) => patch({ locale: e.target.value })}>{Object.entries({ en: 'English', fr: 'Français', es: 'Español', de: 'Deutsch', it: 'Italiano', ja: '日本語', pt: 'Português', nl: 'Nederlands', ru: 'Русский', vi: 'Tiếng Việt', zh: '中文', pl: 'Polski' }).map(([key, label]) => <option key={key} value={key}>{label}</option>)}</select></Field><Field label="Тема встроенного UI"><select value={options.themeMode} onChange={(e) => patch({ themeMode: e.target.value as 'Dark' | 'Light' })}><option value="Dark">Тёмная</option><option value="Light">Светлая</option></select></Field><Field label="Стиль текста легенды"><select value={options.legendTextStyle} onChange={(e) => patch({ legendTextStyle: e.target.value })}>{['Small', 'Body', 'Monospace', 'Button', 'Heading'].map((style) => <option key={style} value={style}>{{ Small: 'Маленький', Body: 'Основной', Monospace: 'Моноширинный', Button: 'Кнопки', Heading: 'Заголовки' }[style]}</option>)}</select></Field></div>
      {options.theme !== undefined && <PluginTheme key={JSON.stringify(options.theme)} value={options.theme} change={(theme) => patch({ theme })} />}
      <Numeric label="Прозрачность встроенных окон" value={Math.round(options.widgetOpacity * 100)} min={0} max={100} change={(value) => patch({ widgetOpacity: value / 100 })} unit="%" />
      <Numeric label="Прозрачность диаграммы Veritas" value={Math.round(options.pieOpacity * 100)} min={0} max={100} change={(value) => patch({ pieOpacity: value / 100 })} unit="%" />
      <Switch label="Подпись Veritas для трансляции" checked={options.streamerMode} disabled={options.suppressNative} onChange={(streamerMode) => patch({ streamerMode })} />
      <TextSetting label="Подпись встроенного UI" value={options.streamerMessage} change={(streamerMessage) => patch({ streamerMessage })} />
      <Switch label="Автоматически показывать / скрывать UI в бою" checked={options.autoShowHide} disabled={options.suppressNative} onChange={(autoShowHide) => patch({ autoShowHide })} hint="Отключено при подавлении встроенного UI." />
      <h4>Шрифты Veritas</h4><div className="vt-form-grid">{Object.entries(options.fontSizes).map(([key, value]) => <Numeric key={key} label={{ Small: 'Маленький', Body: 'Основной', Monospace: 'Моноширинный', Button: 'Кнопки', Heading: 'Заголовки' }[key] ?? key} value={value} min={8} max={48} change={(size) => patch({ fontSizes: { ...options.fontSizes, [key]: size } })} />)}</div>
      <Switch label="Бета-обновления встроенного Veritas" checked={options.beta} onChange={(beta) => patch({ beta })} hint="Бета-сборка может быть несовместима с обычной версией игры. Nous сохраняет проверки официального издателя и SHA-256." />
      <Switch label="Исключение Defender в собственном обновляторе Veritas" checked={options.defenderExclusion} onChange={(defenderExclusion) => patch({ defenderExclusion })} hint="Это параметр плагина. Nous не изменяет настройки Windows Defender." />
      <Button variant="primary" size="sm" onClick={() => void save()}>Записать настройки при закрытой игре</Button>
    </fieldset>}
  </>
}

function PluginTheme({ value, change }: { value: unknown; change: (value: unknown) => void }) {
  const [draft, setDraft] = useState(JSON.stringify(value, null, 2))
  const [error, setError] = useState<string | null>(null)
  const matched = Object.entries(pluginThemes).find(([, theme]) => JSON.stringify(theme.colors) === JSON.stringify(value))?.[0] ?? 'custom'
  return <>
    <Field label="Палитра встроенного UI"><select value={matched} onChange={(e) => { const theme = pluginThemes[e.target.value]; if (theme) { change(theme.colors); setError(null) } }}><option value="custom">Собственная палитра</option>{Object.entries(pluginThemes).map(([key, theme]) => <option key={key} value={key}>{theme.label}</option>)}</select></Field>
    <details className="vt-details"><summary>Редактировать 12 цветовых слотов Veritas</summary><Field label="Палитра в формате плагина" hint="Массив из 12 цветов egui: имена токенов или объект Custom с тремя значениями RGB от 0 до 255."><textarea rows={8} value={draft} onChange={(e) => setDraft(e.target.value)} spellCheck={false} /></Field><Button size="sm" onClick={() => { try { const colors: unknown = JSON.parse(draft); if (!Array.isArray(colors) || colors.length !== 12 || !colors.every((color: unknown) => typeof color === 'string' || (color !== null && typeof color === 'object' && 'Custom' in color && Array.isArray(color.Custom) && color.Custom.length === 3 && color.Custom.every((n: unknown) => typeof n === 'number' && Number.isInteger(n) && n >= 0 && n <= 255)))) throw new Error('Нужны ровно 12 цветовых токенов или RGB-цветов.'); change(colors); setError(null) } catch (e) { setError(`Палитра не применена: ${String(e)}`) } }}>Применить палитру к форме</Button>{error && <p className="vt-error" role="alert">{error}</p>}</details>
  </>
}

function Help() {
  return <div className="vt-help">
    <h4>Как пользоваться оверлеем</h4><p>Установите Veritas только через согласие выше. После запуска игры Nous подключится к локальному потоку событий. Оверлей следует за клиентской областью игры и скрывается, когда окно игры недоступно.</p>
    <p>По умолчанию нажатия проходят в игру. Меняйте представления в этом предпросмотре; открыть настройки можно глобальным сочетанием. Потеря фокуса не завершает бой.</p>
    <h4>Если окно не видно</h4><p>Проверьте переключатель оверлея, его видимость и положение. Для игры используйте оконный или безрамочный режим: эксклюзивный полноэкранный Direct3D может перекрыть внешнее окно Nous.</p>
    <h4>Что означают данные</h4><p>Урон / AV — общий урон, делённый на абсолютное значение действия. Эффективный урон исключает избыточный урон. Ходы с уроном считаются по доступной истории ходов, а не по числу применённых навыков.</p>
    <p>При позднем подключении пропущенные удары и навыки не восстанавливаются. Такие бои помечены как неполные. Разрыв соединения сохраняет последний результат в истории.</p>
    <h4>Отличия от встроенного меню</h4><p>Перенесены таблица персонажей, урон, типы урона, противники, показатели и экспорт. График по ходам / AV доступен дополнительно в Nous: во встроенном меню Veritas 0.2.57 его вызов отключён. Журнал показывает диагностику Nous; внутренние сообщения egui не передаются.</p>
    <p>Поток не содержит reset_action_value. Абсолютное AV действия вычисляется из текущего AV и переданной задержки действия, не ниже текущего AV — как во встроенном интерфейсе. Относительный сброс AV не имитируется. Сброс настроек окна не очищает бой или историю.</p>
    <h4>Настройки и обновления</h4><p>Параметры Nous сохраняются сразу. Параметры самого Veritas записываются отдельной кнопкой при закрытой игре и применяются только после нового запуска. В разделе «Плагин» выберите стабильный или бета-канал, проверьте выпуск, поддерживаемую игру и изменения, затем явно установите обновление. Обновление доступно только после первоначального принятия риска и установки через Nous. Смена канала сбрасывает результат проверки; проверки официального издателя и SHA-256 обязательны для обоих каналов.</p>
  </div>
}

export function VeritasControls() {
  const { state, patch, error: settingsError, saving } = useOverlay()
  const [tab, setTab] = useState<Tab>('Представления')
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [selectedBattle, setSelectedBattle] = useState<Battle | null>(null)
  const [loadingBattle, setLoadingBattle] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [message, setMessage] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [lastExportDirectory, setLastExportDirectory] = useState<string | null>(null)
  const request = useRef(0)
  const native = isTauri()
  const run = (action: () => Promise<unknown>, success?: string) => {
    setBusy(true); setError(null); setMessage(null)
    action().then(() => { if (success) setMessage(success) }).catch((e) => setError(String(e))).finally(() => setBusy(false))
  }
  const select = async (id: number | null) => {
    const sequence = ++request.current
    setSelectedId(id); setSelectedBattle(null); setError(null)
    if (id === null) { setLoadingBattle(false); return }
    setLoadingBattle(true)
    try { const battle = await getBattle(id); if (sequence === request.current) { setSelectedBattle(battle); if (!battle) setError('Бой больше не доступен в истории.') } } catch (e) { if (sequence === request.current) setError(`Не удалось открыть бой: ${String(e)}`) } finally { if (sequence === request.current) setLoadingBattle(false) }
  }
  const saveBattle = (format: 'json' | 'csv' | 'both') => {
    const battle = selectedId === null ? state?.snapshot.battle : selectedBattle
    if (!battle) return
    setBusy(true); setError(null); setMessage(null)
    exportBattle(battle.id, format).then((result) => { setLastExportDirectory(result.directory); setMessage(`Сохранено: ${result.files.join(', ')}. Папка: ${result.directory}`) }).catch((e) => setError(`Экспорт не выполнен: ${String(e)}`)).finally(() => setBusy(false))
  }

  if (!state) return <Panel title="Оверлей Nous"><p className="vt-caption" role="status">Загрузка настроек оверлея…</p>{settingsError && <p className="vt-error" role="alert">{settingsError}</p>}</Panel>
  const { settings, snapshot, presentation } = state
  const battle = selectedId === null ? snapshot.battle : selectedBattle
  return <Panel title="Оверлей Nous" className="vt-controls" aside={<span className="vt-caption" role="status">{saving ? 'Сохранение…' : snapshot.connected ? 'Veritas подключён' : 'Нет соединения'}</span>}>
    <div className="vt-toolbar"><p className="vt-caption">Урон и статистика в отдельном окне с оформлением Nous.</p><div className="vt-actions"><Button size="sm" icon={state.hidden ? Eye : EyeOff} disabled={!native || busy} onClick={() => run(() => overlayAction(state.hidden ? 'show' : 'hide'))}>{state.hidden ? 'Показать окно' : 'Скрыть окно'}</Button><Button size="sm" icon={Settings2} disabled={!native || busy} onClick={() => run(() => overlayAction('settings'))}>Открыть настройки</Button></div></div>
    <p className="vt-status" role="status">{state.replay ? 'Тестовое воспроизведение · без подключения к игре' : presentation.message || (presentation.exclusiveFullscreen ? 'Возможен эксклюзивный полноэкранный режим. Переключите игру в оконный или безрамочный режим.' : !presentation.targetFound ? 'Окно игры не найдено. Настройки и история доступны.' : presentation.visible ? 'Оверлей показан поверх игры.' : state.hidden ? 'Окно оверлея скрыто вручную.' : 'Оверлей сейчас скрыт; ожидается доступное окно игры и соединение.')}</p>
    {(settingsError || error) && <p className="vt-error" role="alert">{settingsError || error}</p>}{message && <p className="vt-success" role="status">{message}</p>}
    <div className="vt-tabs" role="tablist" aria-label="Настройки оверлея">{tabs.map((name, index) => <button key={name} id={`vt-tab-${index}`} role="tab" aria-selected={tab === name} aria-controls="vt-settings-content" tabIndex={tab === name ? 0 : -1} type="button" onClick={() => setTab(name)} onKeyDown={(e) => { const next = e.key === 'ArrowRight' ? (index + 1) % tabs.length : e.key === 'ArrowLeft' ? (index - 1 + tabs.length) % tabs.length : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : null; if (next !== null) { e.preventDefault(); setTab(tabs[next]); document.getElementById(`vt-tab-${next}`)?.focus() } }}>{name}</button>)}</div>
    <div id="vt-settings-content" className="vt-settings-content" role="tabpanel" aria-labelledby={`vt-tab-${tabs.indexOf(tab)}`}>
      {tab === 'Представления' && <ViewSettings settings={settings} patch={patch} />}
      {tab === 'Оформление' && <Appearance settings={settings} patch={patch} />}
      {tab === 'Окно и клавиши' && <WindowSettings settings={settings} patch={patch} run={run} />}
      {tab === 'Экспорт' && <ExportSettings settings={settings} patch={patch} />}
      {tab === 'Плагин' && <PluginSettings />}
      {tab === 'Справка' && <Help />}
    </div>
    <div className="vt-preview-heading"><h3>Предпросмотр и история</h3><span className="vt-caption">Те же представления, что в окне игры</span></div>
    <Field label="Показать бой"><select value={selectedId ?? 'current'} disabled={loadingBattle} onChange={(e) => void select(e.target.value === 'current' ? null : Number(e.target.value))}><option value="current">{snapshot.active ? 'Текущий бой' : 'Последний полученный бой'}</option>{snapshot.history.map((entry) => <option key={entry.id} value={entry.id}>Бой {entry.id} · Этап {entry.stageId || 'неизвестен'} · {formatNumber(entry.totalDamage)} урона{entry.partial ? ' · неполный' : ''}{entry.interrupted ? ' · прерван' : ''}</option>)}</select></Field>
    <div className="vt-preview" aria-busy={loadingBattle}>{loadingBattle ? <p className="vt-empty" role="status">Загрузка боя…</p> : <BattleView battle={battle} snapshot={snapshot} settings={settings} patch={patch} />}</div>
    <div className="vt-toolbar"><p className="vt-caption">{battle ? `Этап ${battle.stageId || 'неизвестен'} · ${battle.endedAt === null ? 'Запись боя' : 'Бой сохранён'}${battle.partial ? ' · Неполные данные' : ''}` : 'Записи появятся после первого боя.'}</p><div className="vt-actions"><Button size="sm" disabled={!native || !battle || busy} onClick={() => saveBattle('json')}>JSON</Button><Button size="sm" disabled={!native || !battle || busy} onClick={() => saveBattle('csv')}>CSV</Button><Button size="sm" icon={Download} disabled={!native || !battle || busy} onClick={() => saveBattle('both')}>{busy ? 'Обработка…' : 'Оба формата'}</Button></div></div>
    {(lastExportDirectory || settings.exportDirectory) && <Button size="sm" variant="ghost" disabled={!native || busy} onClick={() => run(() => openPath(lastExportDirectory || settings.exportDirectory!))}>Открыть папку экспорта</Button>}
  </Panel>
}
