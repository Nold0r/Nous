import { useState } from 'react'
import type { CSSProperties, ReactNode } from 'react'
import { Minus, Plus, RotateCcw, Users } from 'lucide-react'
import { Button } from '@app/ui'
import { useBattleIcons } from './icons'
import { categoryLabel, characterValue, columnLabels, damageCategories, dpav, effective, finite, formatCompact, formatNumber, formatShare, projectedAv, propertyLabel, seriesColors, sortedAvatars, themeVariables, viewLabels } from './presentation'
import type { Battle, Combatant, OverlaySettings, Snapshot, ViewKey } from './types'

type Patch = (changes: Partial<OverlaySettings>) => void

function Avatar({ avatar }: { avatar: Combatant }) {
  const image = useBattleIcons().character(avatar)
  return <span className="vt-avatar" title={avatar.name}>
    {image ? <img src={image} alt="" width="36" height="36" /> : <Users size={20} aria-hidden />}
    <span className="vt-avatar__name">{avatar.name || `Персонаж ${avatar.id}`}{avatar.dead && <small>Выведен из боя</small>}</span>
  </span>
}

function Section({ name, children }: { name: ViewKey; children: ReactNode }) {
  return <section className={`vt-section vt-section--${name}`} aria-label={viewLabels[name]}>
    <h3>{viewLabels[name]}</h3>{children}
  </section>
}

function MetricTabs({ settings, patch }: { settings: OverlaySettings; patch: Patch }) {
  return <div className="vt-segments" aria-label="Показатель урона">
    <button type="button" aria-pressed={settings.metric === 'damage'} onClick={() => patch({ metric: 'damage' })}>Урон</button>
    <button type="button" aria-pressed={settings.metric === 'dpav'} onClick={() => patch({ metric: 'dpav' })}>Урон / AV</button>
  </div>
}

function CharacterTable({ battle, settings }: { battle: Battle; settings: OverlaySettings }) {
  return <div className="vt-table-scroll"><table className="vt-table vt-character-table" style={{ fontSize: 'var(--vt-legend-size)' }}>
    <thead><tr><th scope="col">Персонаж</th>{settings.columns.map((column) => <th scope="col" key={column}>{columnLabels[column]}</th>)}</tr></thead>
    <tbody>{sortedAvatars(battle, settings.metric).map((avatar) => <tr key={avatar.id}>
      <th scope="row"><Avatar avatar={avatar} /></th>
      {settings.columns.map((column) => <td key={column}>{characterValue(avatar, column, battle)}</td>)}
    </tr>)}</tbody>
    <tfoot><tr><th scope="row">Вся команда</th>{settings.columns.map((column) => <td key={column}>{column === 'share' ? formatShare(battle.totalDamage, battle.totalDamage) : column === 'turns' ? '—' : formatNumber(
      column === 'dpav' ? dpav(battle.totalDamage, battle.actionValue) : column === 'effective' ? battle.avatars.reduce((sum, avatar) => sum + effective(avatar), 0) : column === 'overkill' ? battle.avatars.reduce((sum, avatar) => sum + avatar.overkill, 0) : battle.totalDamage,
    )}</td>)}</tr></tfoot>
  </table></div>
}

function CharacterMeters({ battle, settings, patch }: { battle: Battle; settings: OverlaySettings; patch: Patch }) {
  const avatars = sortedAvatars(battle, settings.metric)
  const max = Math.max(1, ...avatars.map((avatar) => avatar.damage))
  return <>
    <MetricTabs settings={settings} patch={patch} />
    <div className="vt-meters">{avatars.map((avatar, i) => <div className="vt-meter-row" key={avatar.id} title={`${avatar.name}: ${formatNumber(avatar.damage)} урона, ${formatNumber(effective(avatar))} эффективного, ${formatNumber(avatar.overkill)} избыточного, ${formatNumber(dpav(avatar.damage, battle.actionValue))} урона / AV`}>
      <div className="vt-meter-row__label"><Avatar avatar={avatar} /><span>{formatCompact(settings.metric === 'damage' ? avatar.damage : dpav(avatar.damage, battle.actionValue))}<small>{formatShare(avatar.damage, battle.totalDamage)}</small></span></div>
      <div className="vt-meter" aria-label={`${avatar.name}: ${formatShare(avatar.damage, battle.totalDamage)} урона команды`}>
        <span style={{ width: `${Math.max(0, Math.min(100, avatar.damage / max * 100))}%`, background: seriesColors[i % seriesColors.length] }}>
          <i style={{ width: `${Math.max(0, Math.min(100, avatar.damage > 0 ? avatar.overkill / avatar.damage * 100 : 0))}%` }} />
        </span>
      </div>
    </div>)}</div>
    <p className="vt-caption">Отмеченный участок полосы — избыточный урон.</p>
  </>
}

function CategoryChart({ categories, pie, opacity }: { categories: [string, number][]; pie: boolean; opacity: number }) {
  const total = categories.reduce((sum, [, value]) => sum + value, 0)
  const max = Math.max(1, ...categories.map(([, value]) => value))
  if (total <= 0) return <p className="vt-empty">Для выбранного персонажа ещё нет урона.</p>
  return pie ? <svg className="vt-pie" viewBox="0 0 240 200" role="img" aria-label="Распределение урона по типам">
    {categories.map(([name, value], i) => {
      const start = -Math.PI / 2 + categories.slice(0, i).reduce((sum, [, damage]) => sum + damage, 0) / total * Math.PI * 2
      const end = start + value / total * Math.PI * 2
      const label = `${categoryLabel(name)}: ${formatNumber(value)} (${formatShare(value, total)})`
      const color = seriesColors[i % seriesColors.length]
      if (value / total >= .999999) return <circle key={name} cx="120" cy="100" r="80" fill={color} fillOpacity={opacity}><title>{label}</title></circle>
      const path = `M120 100 L${120 + 80 * Math.cos(start)} ${100 + 80 * Math.sin(start)} A80 80 0 ${end - start > Math.PI ? 1 : 0} 1 ${120 + 80 * Math.cos(end)} ${100 + 80 * Math.sin(end)} Z`
      return <path key={name} d={path} fill={color} fillOpacity={opacity} stroke="var(--vt-surface)" strokeWidth="2"><title>{label}</title></path>
    })}
  </svg> : <div className="vt-category-bars">{categories.map(([name, value], i) => <div key={name}>
    <div className="vt-inline"><span>{categoryLabel(name)}</span><span>{formatCompact(value)} · {formatShare(value, total)}</span></div>
    <div className="vt-meter"><span style={{ width: `${value / max * 100}%`, background: seriesColors[i % seriesColors.length] }} /></div>
  </div>)}</div>
}

function Breakdown({ battle, settings, patch }: { battle: Battle; settings: OverlaySettings; patch: Patch }) {
  const character = battle.avatars.find((avatar) => avatar.id === settings.characterId) ?? battle.avatars[0]
  const selectedId = settings.breakdownScope === 'character' ? character?.id ?? null : null
  const categories = damageCategories(battle, selectedId)
  const selectedDamage = selectedId === null ? battle.totalDamage : character?.damage ?? 0
  return <>
    <div className="vt-inline vt-inline--wrap">
      <div className="vt-segments" aria-label="Область типов урона">
        <button type="button" aria-pressed={settings.breakdownScope === 'team'} onClick={() => patch({ breakdownScope: 'team' })}>Команда</button>
        <button type="button" aria-pressed={settings.breakdownScope === 'character'} onClick={() => patch({ breakdownScope: 'character' })}>Персонаж</button>
      </div>
      <label className="vt-inline vt-caption">Диаграмма <select aria-label="Тип диаграммы" value={settings.breakdownChart} onChange={(e) => patch({ breakdownChart: e.target.value as OverlaySettings['breakdownChart'] })}><option value="pie">Круговая</option><option value="bar">Полосы</option></select></label>
    </div>
    {settings.breakdownScope === 'character' && <label className="vt-field">Персонаж<select value={selectedId ?? ''} onChange={(e) => patch({ characterId: Number(e.target.value) })}>{battle.avatars.map((avatar) => <option key={avatar.id} value={avatar.id}>{avatar.name}</option>)}</select></label>}
    <p className="vt-caption">{selectedId === null ? 'Вся команда' : character?.name} · {formatNumber(selectedDamage)} · {formatShare(selectedDamage, battle.totalDamage)} команды</p>
    <CategoryChart categories={categories} pie={settings.breakdownChart === 'pie'} opacity={settings.pieOpacity} />
    {settings.showLegend && <div className="vt-chart-legend">{categories.map(([name, value], i) => <span key={name} title={formatNumber(value)}><i style={{ background: seriesColors[i % seriesColors.length] }} />{categoryLabel(name)}</span>)}</div>}
    {settings.showBreakdownTable && <div className="vt-table-scroll"><table className="vt-table"><thead><tr><th scope="col">Тип</th><th scope="col">Урон</th><th scope="col">Доля выборки</th><th scope="col">Доля команды</th></tr></thead><tbody>{categories.map(([name, damage]) => <tr key={name}><th scope="row">{categoryLabel(name)}</th><td>{formatNumber(damage)}</td><td>{formatShare(damage, selectedDamage)}</td><td>{formatShare(damage, battle.totalDamage)}</td></tr>)}</tbody></table></div>}
    {settings.showBreakdownTable && selectedId === null && <details className="vt-details"><summary>Вклад персонажей</summary><table className="vt-table"><thead><tr><th scope="col">Персонаж</th><th scope="col">Урон</th><th scope="col">Доля команды</th></tr></thead><tbody>{battle.avatars.map((avatar) => <tr key={avatar.id}><th scope="row">{avatar.name}</th><td>{formatNumber(avatar.damage)}</td><td>{formatShare(avatar.damage, battle.totalDamage)}</td></tr>)}</tbody></table></details>}
  </>
}

function Enemies({ battle }: { battle: Battle }) {
  const icons = useBattleIcons()
  if (!battle.enemies.length) return <p className="vt-empty">Данные противников ещё не получены.</p>
  return <div className="vt-enemies">{battle.enemies.map((enemy) => {
    const hp = enemy.properties.CurrentHP
    const max = enemy.properties.MaxHP
    const absoluteAv = enemy.dead ? null : projectedAv(battle.actionValue, enemy.properties.ActionDelay)
    return <details key={enemy.uid} className="vt-details"><summary><span className="vt-avatar">{icons.enemy(enemy) ? <img src={icons.enemy(enemy)} alt="" width="36" height="36" /> : null}<span className="vt-avatar__name">{enemy.name || `Противник ${enemy.id}`}{enemy.dead ? ' · Повержен' : ''}</span></span><span>{hp === undefined ? 'HP неизвестны' : `${formatNumber(hp)}${max ? ` / ${formatNumber(max)} · ${formatShare(hp, max)}` : ''}`}</span></summary>
      {max > 0 && hp !== undefined && <div className="vt-meter"><span style={{ width: `${Math.max(0, Math.min(100, hp / max * 100))}%`, background: 'var(--danger)' }} /></div>}
      <dl className="vt-property-list">{Object.entries(enemy.properties).map(([name, value]) => <div key={name}><dt>{propertyLabel(name)}</dt><dd>{formatNumber(value)}</dd></div>)}{absoluteAv !== null && <div><dt title="Текущее AV плюс задержка действия; не меньше текущего AV">Абсолютное AV действия</dt><dd>{formatNumber(absoluteAv)}</dd></div>}</dl>
    </details>
  })}</div>
}

function Metrics({ battle }: { battle: Battle }) {
  return <>
    <dl className="vt-metrics"><div><dt>Общий урон</dt><dd>{formatNumber(battle.totalDamage)}</dd></div><div><dt>Значение действия (AV)</dt><dd>{formatNumber(battle.actionValue)}</dd></div><div><dt>Урон / AV</dt><dd>{formatNumber(dpav(battle.totalDamage, battle.actionValue))}</dd></div><div><dt>Волна / цикл</dt><dd>{battle.wave}{battle.maxWaves ? ` / ${battle.maxWaves}` : ''} · {battle.cycle}{battle.maxCycles ? ` / ${battle.maxCycles}` : ''}</dd></div></dl>
    <details className="vt-details"><summary>Значения персонажей</summary><div className="vt-table-scroll"><table className="vt-table"><thead><tr><th scope="col">Персонаж</th><th scope="col">Урон</th><th scope="col">Урон / AV</th><th scope="col">Задержка действия</th><th scope="col" title="Текущее AV плюс переданная задержка действия; не меньше текущего AV">Абс. AV действия</th></tr></thead><tbody>{battle.avatars.map((avatar) => {
      const absoluteAv = avatar.dead ? null : projectedAv(battle.actionValue, avatar.properties.ActionDelay)
      return <tr key={avatar.id}><th scope="row">{avatar.name}</th><td>{formatNumber(avatar.damage)}</td><td>{formatNumber(dpav(avatar.damage, battle.actionValue))}</td><td>{avatar.properties.ActionDelay === undefined ? '—' : formatNumber(avatar.properties.ActionDelay)}</td><td>{absoluteAv === null ? '—' : formatNumber(absoluteAv)}</td></tr>
    })}</tbody></table></div></details>
    <p className="vt-caption">AV действия вычисляется как текущее AV плюс задержка действия, не ниже текущего AV. Относительный сброс AV не передаётся плагином.</p>
    {battle.actionValue <= 0 && <p className="vt-caption">Урон / AV появится после продвижения шкалы действий; при AV = 0 показан 0.</p>}
  </>
}

function DamageGraph({ battle, settings, patch }: { battle: Battle; settings: OverlaySettings; patch: Patch }) {
  const [zoom, setZoom] = useState(1)
  const all = settings.graphUnit === 'turn' ? battle.turnHistory : battle.avHistory
  const points = all.map((turn, i) => ({ turn, x: settings.graphUnit === 'turn' ? i + 1 : finite(turn.actionValue) })).slice(-Math.max(2, Math.ceil(all.length / zoom)))
  const minX = points[0]?.x ?? 0
  const maxX = Math.max(minX + 1, points.at(-1)?.x ?? 1)
  const maxY = Math.max(1, ...points.flatMap(({ turn }) => turn.avatarsTurnDamage.map(finite)))
  return <>
    <div className="vt-inline vt-inline--wrap"><label className="vt-inline vt-caption">Ось X<select aria-label="Единица графика" value={settings.graphUnit} onChange={(e) => patch({ graphUnit: e.target.value as OverlaySettings['graphUnit'] })}><option value="turn">Ход</option><option value="action-value">Значение действия</option></select></label>
      <div className="vt-inline"><Button size="sm" variant="ghost" icon={Minus} title="Отдалить график" aria-label="Отдалить график" disabled={zoom <= 1} onClick={() => setZoom(Math.max(1, zoom / 2))} /><Button size="sm" variant="ghost" icon={Plus} title="Приблизить последние ходы" aria-label="Приблизить последние ходы" disabled={zoom >= 8} onClick={() => setZoom(Math.min(8, zoom * 2))} /><Button size="sm" variant="ghost" icon={RotateCcw} title="Сбросить график" aria-label="Сбросить график" onClick={() => setZoom(1)} /></div>
    </div>
    {points.length ? <svg className="vt-line-chart" viewBox="0 0 460 190" role="img" aria-label={`Урон по ${settings.graphUnit === 'turn' ? 'ходам' : 'AV'}, масштаб ${zoom}`} onDoubleClick={() => setZoom(1)}>
      {[0, .5, 1].map((fraction) => <g key={fraction}><line x1="70" y1={155 - 135 * fraction} x2="450" y2={155 - 135 * fraction} stroke="var(--line)" /><text x="64" y={159 - 135 * fraction} textAnchor="end">{formatCompact(maxY * fraction)}</text></g>)}
      <text x="70" y="180">{formatNumber(minX)}</text><text x="450" y="180" textAnchor="end">{formatNumber(maxX)}</text>
      {battle.avatars.map((avatar, avatarIndex) => {
        const data = points.map(({ turn, x }) => ({ x: 70 + (x - minX) / (maxX - minX) * 380, y: 155 - finite(turn.avatarsTurnDamage[avatarIndex]) / maxY * 135, damage: finite(turn.avatarsTurnDamage[avatarIndex]) }))
        return <g key={avatar.id} stroke={seriesColors[avatarIndex % seriesColors.length]}><polyline points={data.map(({ x, y }) => `${x},${y}`).join(' ')} fill="none" strokeWidth="2"><title>{avatar.name}</title></polyline>{data.map(({ x, y, damage }, i) => <circle key={i} cx={x} cy={y} r="3" fill={seriesColors[avatarIndex % seriesColors.length]}><title>{avatar.name}: {formatNumber(damage)} · {formatNumber(points[i].x)}</title></circle>)}</g>
      })}
    </svg> : <p className="vt-empty">График появится после получения истории ходов.</p>}
    <div className="vt-chart-legend">{battle.avatars.map((avatar, i) => <span key={avatar.id}><i style={{ background: seriesColors[i % seriesColors.length] }} />{avatar.name}</span>)}</div>
    <p className="vt-caption">Урон за ход, без накопления. Приближение показывает последние ходы. Двойной щелчок сбрасывает график.</p>
  </>
}

/** One view's content; shared by the widgets over the game and the preview. */
function ViewContent({ name, battle, snapshot, settings, patch }: { name: ViewKey; battle: Battle | null; snapshot: Snapshot; settings: OverlaySettings; patch: Patch }) {
  if (name === 'logs') return <>{snapshot.diagnostics.length ? <ol className="vt-log">{snapshot.diagnostics.map((line, i) => <li key={`${i}-${line}`}>{line}</li>)}</ol> : <p className="vt-empty">Сообщений пока нет.</p>}</>
  if (!battle) return <p className="vt-empty">Появится в бою.</p>
  switch (name) {
    case 'legend': return <CharacterTable battle={battle} settings={settings} />
    case 'characters': return <CharacterMeters battle={battle} settings={settings} patch={patch} />
    case 'breakdown': return <Breakdown battle={battle} settings={settings} patch={patch} />
    case 'enemies': return <Enemies battle={battle} />
    case 'metrics': return <Metrics battle={battle} />
    case 'graph': return <DamageGraph key={battle.id} battle={battle} settings={settings} patch={patch} />
  }
}

/** A single widget over the game. */
export function Widget({ name, battle, snapshot, settings, patch }: { name: ViewKey; battle: Battle | null; snapshot: Snapshot; settings: OverlaySettings; patch: Patch }) {
  return <div className="vt-battle" style={themeVariables(settings) as CSSProperties}>
    <Section name={name}><ViewContent name={name} battle={battle} snapshot={snapshot} settings={settings} patch={patch} /></Section>
  </div>
}

export function BattleView({ battle, snapshot, settings, patch }: { battle: Battle | null; snapshot: Snapshot; settings: OverlaySettings; patch: Patch }) {
  return <div className="vt-battle" style={themeVariables(settings) as CSSProperties}>
    {battle?.partial && <p className="vt-notice" role="status">Неполные данные: подключение произошло после начала боя. Пропущенные удары не восстанавливаются.</p>}
    {battle?.interrupted && <p className="vt-notice" role="status">Запись прервана. Показан последний полученный результат.</p>}
    {!battle && <div className="vt-empty-state"><Users size={30} aria-hidden /><h3>{snapshot.connected ? 'Ожидание боя' : 'Нет соединения с Veritas'}</h3><p>{snapshot.connected ? 'Персонажи и урон появятся после начала боя.' : 'Запустите игру с ранее включённым плагином. Пока соединения нет, сохранённые бои доступны в истории.'}</p></div>}
    {battle && !Object.values(settings.views).some(Boolean) && <p className="vt-empty">Все представления скрыты. Включите нужные в настройках оверлея.</p>}
    {battle && (Object.keys(viewLabels) as ViewKey[]).filter((name) => name !== 'logs' && settings.views[name]).map((name) => <Section key={name} name={name}><ViewContent name={name} battle={battle} snapshot={snapshot} settings={settings} patch={patch} /></Section>)}
    {settings.views.logs && <Section name="logs"><ViewContent name="logs" battle={battle} snapshot={snapshot} settings={settings} patch={patch} /></Section>}
    {settings.streamerMode && <footer className="vt-streamer">{settings.streamerMessage || 'Nous'}</footer>}
  </div>
}
