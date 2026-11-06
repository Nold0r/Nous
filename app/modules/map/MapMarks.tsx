import type { ReactNode } from 'react'
import { Bug, Check, Ghost, Puzzle } from 'lucide-react'
import type { MarkKind } from './maps'
import { groupOf, isDone, isPresent, type Mark, type MarkFilter, type Progress } from './marks'

// Chests and puzzles on the map. Puzzles are what the game marks with its
// puzzle icon, and reward chests with no icon (see scripts/map-markers.py).

const TITLES: Record<MarkKind, string> = {
  basic: 'Обычный сундук',
  bountiful: 'Ценный сундук',
  precious: 'Драгоценный сундук',
  puzzle: 'Головоломка',
  nymph: 'Нимфа',
  spirithief: 'Душекрад',
}

const DONE: Record<MarkKind, string> = {
  basic: 'открыт',
  bountiful: 'открыт',
  precious: 'открыт',
  puzzle: 'решена',
  nymph: 'поймана',
  spirithief: 'пойман',
}

function MarkIcon({ mark }: { mark: Mark }) {
  if (mark.kind === 'nymph') return <Bug size={12} strokeWidth={2.4} />
  if (mark.kind === 'spirithief') return <Ghost size={12} strokeWidth={2.4} />
  if (mark.kind === 'puzzle' && !mark.reward) return <Puzzle size={12} strokeWidth={2.4} />
  return <ChestIcon />
}

function ChestIcon() {
  return (
    <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden>
      <path d="M2 6.5C2 4 4 2.5 8 2.5S14 4 14 6.5V7H2z" />
      <path d="M2 8h12v5a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z" />
      <rect x="7" y="7" width="2" height="3" rx="0.5" className="map-mark__lock" />
    </svg>
  )
}

/** One mark, at screen px. */
export function MarkPin({ mark, left, top, opened }: { mark: Mark; left: number; top: number; opened: boolean }) {
  const kind = mark.kind
  const title = mark.reward ? 'Награда головоломки' : mark.pair ? 'Головоломка: начало' : TITLES[kind]
  return (
    <div
      className={`map-mark map-mark--${kind}${mark.reward ? ' map-mark--reward' : ''}${opened ? ' is-opened' : ''}`}
      style={{ left, top }}
      title={title + (opened ? ` — ${DONE[kind]}` : '')}
    >
      <MarkIcon mark={mark} />
      {opened && <Check size={10} strokeWidth={3.5} className="map-mark__check" />}
    </div>
  )
}

/** Toggles over the map: chests, puzzles, opened ones; with counts. */
export function MarkLegend({
  marks,
  filter,
  onChange,
  progress,
}: {
  marks: Mark[]
  filter: MarkFilter
  onChange: (f: MarkFilter) => void
  progress: Progress
}) {
  const listOf = (group: keyof MarkFilter) => marks.filter((m) => groupOf(m) === group && !m.reward && isPresent(m, progress))
  const count = (group: keyof MarkFilter) => {
    const list = listOf(group)
    const known = list.map((m) => isDone(m, progress)).filter((d) => d != null)
    return known.length ? `${known.filter(Boolean).length}/${list.length}` : `${list.length}`
  }
  const hasNymphs = listOf('nymphs').length > 0
  const hasSpirits = listOf('spirits').length > 0
  if (!marks.length) return null
  const toggle = (key: keyof MarkFilter, label: string, icon: ReactNode, extra?: string) => (
    <button
      type="button"
      aria-pressed={filter[key]}
      className={filter[key] ? 'is-on' : undefined}
      onClick={() => onChange({ ...filter, [key]: !filter[key] })}
    >
      {icon}
      <span>{label}</span>
      {extra && <span className="map-legend__count">{extra}</span>}
    </button>
  )
  return (
    <div className="map-legend" onPointerDown={(e) => e.stopPropagation()} onDoubleClick={(e) => e.stopPropagation()}>
      {toggle('chests', 'Сундуки', <span className="map-legend__icon map-mark--precious"><ChestIcon /></span>, count('chests'))}
      {toggle('puzzles', 'Головоломки', <span className="map-legend__icon map-mark--puzzle"><Puzzle size={12} strokeWidth={2.4} /></span>, count('puzzles'))}
      {hasNymphs &&
        toggle('nymphs', 'Нимфы', <span className="map-legend__icon map-mark--nymph"><Bug size={12} strokeWidth={2.4} /></span>, count('nymphs'))}
      {hasSpirits &&
        toggle('spirits', 'Душекрады', <span className="map-legend__icon map-mark--spirithief"><Ghost size={12} strokeWidth={2.4} /></span>, count('spirits'))}
      {toggle('opened', 'Открытые', <span className="map-legend__icon"><Check size={12} strokeWidth={3} /></span>)}
      {!progress.opened && <p className="map-legend__hint">Откройте карту в игре, чтобы Nous узнал, что уже открыто</p>}
    </div>
  )
}
