import { Fragment, type ReactNode } from 'react'
import { BookOpen, Check } from 'lucide-react'
import { mapUrl, type MarkKind } from './maps'
import { groupOf, isDone, isPresent, type Mark, type MarkFilter, type Progress } from './marks'

// Chests, puzzles and collectibles on the map, drawn with the game's own map
// icons. What each kind is in the game's data: scripts/map-markers.py.

const TITLES: Record<MarkKind, string> = {
  basic: 'Обычный сундук',
  bountiful: 'Ценный сундук',
  precious: 'Драгоценный сундук',
  puzzle: 'Головоломка',
  nymph: 'Нимфа',
  origami: 'Птица-оригами',
  wisp: 'Безымянный эфемер',
  rabbit: 'Кролик звёздного пути',
  anchor: 'Пространственный якорь',
  book: 'Книга или заметка',
  bubble: 'Пузырь памяти',
  outpost: 'Место сдачи',
  spirithief: 'Душекрад',
}

const DONE: Record<MarkKind, string> = {
  basic: 'открыт',
  bountiful: 'открыт',
  precious: 'открыт',
  puzzle: 'решена',
  nymph: 'поймана',
  origami: 'поймана',
  wisp: 'найден',
  rabbit: 'найден',
  anchor: 'активирован',
  book: 'прочитана',
  bubble: 'просмотрен',
  outpost: '',
  spirithief: 'пойман',
}

/** Toggles for what only some maps have, shown where there is any. */
const EXTRA: { group: keyof MarkFilter; kind: MarkKind; label: string }[] = [
  { group: 'anchors', kind: 'anchor', label: 'Якоря' },
  { group: 'books', kind: 'book', label: 'Книги' },
  { group: 'bubbles', kind: 'bubble', label: 'Пузыри памяти' },
  { group: 'nymphs', kind: 'nymph', label: 'Нимфы' },
  { group: 'spirits', kind: 'spirithief', label: 'Душекрады' },
  { group: 'origami', kind: 'origami', label: 'Птицы-оригами' },
  { group: 'wisps', kind: 'wisp', label: 'Эфемеры' },
  { group: 'rabbits', kind: 'rabbit', label: 'Кролики' },
  { group: 'outposts', kind: 'outpost', label: 'Места сдачи' },
]

/** The game's map icon for each kind (SpriteOutput/MapPics/NaviIcons, in public/maps/icons). */
const GAME_ICONS: Record<Exclude<MarkKind, 'book'>, string> = {
  // the game's chest icon, tinted by tier (blue, purple, gold)
  basic: 'IconMapChest-basic',
  bountiful: 'IconMapChest-bountiful',
  precious: 'IconMapChest-precious',
  puzzle: 'IconMapPlay6',
  nymph: 'IconMapSpecimenUnknown',
  origami: 'IconMapOrigamiBird',
  wisp: 'IconMapSticker',
  rabbit: 'IconMapDustSpirit',
  anchor: 'IconMapMazePortal2',
  bubble: 'IconMapMemoryBubble',
  outpost: 'IconMapToken',
  spirithief: 'IconMapThiefElfControl',
}

/** The game swaps these icons when done: a caught specimen, an active anchor. */
const DONE_ICONS: Partial<Record<MarkKind, string>> = {
  nymph: 'IconMapSpecimen',
  anchor: 'IconMapMazePortal',
}

/** Kinds whose done icon says it all: not greyed, no tick. */
const SHOWN_DONE: MarkKind[] = ['anchor']

function MarkIcon({ kind, reward = false, done = false }: { kind: MarkKind; reward?: boolean; done?: boolean }) {
  if (kind === 'book') return <BookOpen size={12} strokeWidth={2.4} />
  const icon = reward ? 'IconMapBox' : (done && DONE_ICONS[kind]) || GAME_ICONS[kind]
  return <img src={mapUrl(`icons/${icon}.webp`)} alt="" draggable={false} />
}

/** One mark, at screen px. */
export function MarkPin({ mark, left, top, opened }: { mark: Mark; left: number; top: number; opened: boolean }) {
  const kind = mark.kind
  const title = mark.name || (mark.reward ? 'Награда головоломки' : mark.pair ? 'Головоломка: начало' : TITLES[kind])
  const ticked = opened && !SHOWN_DONE.includes(kind)
  return (
    <div
      className={`map-mark map-mark--${kind}${mark.reward ? ' map-mark--reward' : ''}${ticked ? ' is-opened' : ''}`}
      style={{ left, top }}
      title={title + (opened ? ` — ${DONE[kind]}` : '')}
    >
      <MarkIcon kind={kind} reward={mark.reward} done={opened} />
      {ticked && <Check size={10} strokeWidth={3.5} className="map-mark__check" />}
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
      {toggle('chests', 'Сундуки', <span className="map-legend__icon"><MarkIcon kind="basic" /></span>, count('chests'))}
      {toggle('puzzles', 'Головоломки', <span className="map-legend__icon"><MarkIcon kind="puzzle" /></span>, count('puzzles'))}
      {EXTRA.filter((e) => listOf(e.group).length > 0).map((e) => (
        <Fragment key={e.group}>
          {toggle(
            e.group,
            e.label,
            <span className="map-legend__icon">
              <MarkIcon kind={e.kind} done={e.kind === 'anchor'} />
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
