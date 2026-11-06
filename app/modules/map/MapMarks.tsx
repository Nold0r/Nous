import { Fragment, type ReactNode } from 'react'
import { Bird, Bug, Check, Ghost, Puzzle, Rabbit } from 'lucide-react'
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
  origami: 'Птица-оригами',
  rabbit: 'Кролик звёздного пути',
  spirithief: 'Душекрад',
}

const DONE: Record<MarkKind, string> = {
  basic: 'открыт',
  bountiful: 'открыт',
  precious: 'открыт',
  puzzle: 'решена',
  nymph: 'поймана',
  origami: 'поймана',
  rabbit: 'найден',
  spirithief: 'пойман',
}

/** Toggles for what only some maps have, shown where there is any. */
const EXTRA: { group: keyof MarkFilter; kind: MarkKind; label: string }[] = [
  { group: 'nymphs', kind: 'nymph', label: 'Нимфы' },
  { group: 'spirits', kind: 'spirithief', label: 'Душекрады' },
  { group: 'origami', kind: 'origami', label: 'Птицы-оригами' },
  { group: 'rabbits', kind: 'rabbit', label: 'Кролики' },
]

function MarkIcon({ kind, reward = false }: { kind: MarkKind; reward?: boolean }) {
  if (kind === 'nymph') return <Bug size={12} strokeWidth={2.4} />
  if (kind === 'origami') return <Bird size={12} strokeWidth={2.4} />
  if (kind === 'rabbit') return <Rabbit size={12} strokeWidth={2.4} />
  if (kind === 'spirithief') return <Ghost size={12} strokeWidth={2.4} />
  if (kind === 'puzzle' && !reward) return <Puzzle size={12} strokeWidth={2.4} />
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
      <MarkIcon kind={kind} reward={mark.reward} />
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
      {EXTRA.filter((e) => listOf(e.group).length > 0).map((e) => (
        <Fragment key={e.group}>
          {toggle(
            e.group,
            e.label,
            <span className={`map-legend__icon map-mark--${e.kind}`}>
              <MarkIcon kind={e.kind} />
            </span>,
            count(e.group),
          )}
        </Fragment>
      ))}
      {toggle('opened', 'Открытые', <span className="map-legend__icon"><Check size={12} strokeWidth={3} /></span>)}
      {!progress.opened && <p className="map-legend__hint">Откройте карту в игре, чтобы Nous узнал, что уже открыто</p>}
    </div>
  )
}
