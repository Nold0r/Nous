import { useMemo, useState } from 'react'
import { NotebookPen } from 'lucide-react'
import {
  FilterMenu,
  FilterTags,
  Grid,
  ItemCard,
  SearchInput,
  Select,
  StatIcon,
  type FilterOption,
  type FilterState,
} from '@app/ui'
import { RelicBadges, RelicModal, RelicStats, RelicWearer } from '@app/components/relic'
import { isNewRelic, useNotes, useNow, type Inventory } from '@app/lib/inventory'
import { useProfile } from '@app/lib/profile'
import {
  RELIC_SLOTS,
  characterName,
  resUrl,
  slotMainStats,
  slotName,
  slotSubStats,
  viewRelic,
  type GameTables,
  type RelicView,
  type Stat,
} from '@app/lib/game-tables'

type Sort = 'level' | 'new' | 'set'

const SORTS = [
  { value: 'level', label: 'По уровню' },
  { value: 'new', label: 'Сначала новые' },
  { value: 'set', label: 'По набору' },
] as const

const STATUS: FilterOption[] = [
  { value: 'new', label: 'Новая (за сутки)' },
  { value: 'locked', label: 'Заблокирована' },
  { value: 'discarded', label: 'На разбор' },
  { value: 'equipped', label: 'Надета' },
  { value: 'note', label: 'С заметкой' },
]

type Filters = Record<'slot' | 'set' | 'main' | 'sub' | 'status', FilterState>
const NO_FILTERS: Filters = { slot: {}, set: {}, main: {}, sub: {}, status: {} }

/** One value per relic (slot, set, main stat): white list and black list. */
function passOne(f: FilterState, value: string): boolean {
  if (f[value] === 'exclude') return false
  return !Object.values(f).includes('include') || f[value] === 'include'
}

/** Several values per relic (substats, statuses): has every included one, none of the excluded. */
function passAll(f: FilterState, values: string[]): boolean {
  return Object.entries(f).every(([v, mode]) => values.includes(v) === (mode === 'include'))
}

/** Stat options; "%" tells HP% from flat HP and the like. */
function statOptions(stats: Stat[]): FilterOption[] {
  const byProperty = new Map(stats.map((s) => [s.property, s]))
  const names = [...byProperty.values()].map((s) => s.name)
  return [...byProperty.values()]
    .map((s) => ({
      value: s.property,
      label: s.percent && names.filter((n) => n === s.name).length > 1 ? `${s.name} %` : s.name,
      icon: <StatIcon icon={s.icon} size={16} />,
    }))
    .sort((a, b) => a.label.localeCompare(b.label, 'ru'))
}

export function RelicsTab({ inventory, tables }: { inventory: Inventory; tables: GameTables }) {
  const notes = useNotes()
  const profile = useProfile()
  const [filters, setFilters] = useState<Filters>(NO_FILTERS)
  const [sort, setSort] = useState<Sort>('level')
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState<number | null>(null)
  const now = useNow()

  const relics = useMemo(
    () => Object.values(inventory.relics).flatMap((r) => viewRelic(tables, r) ?? []),
    [inventory.relics, tables],
  )

  // stats the chosen slots can have (all slots when none is chosen), from the game tables
  const possible = useMemo(() => {
    const slots = RELIC_SLOTS.map((x) => x.id).filter((id) => passOne(filters.slot, id))
    return {
      main: new Set(slots.flatMap((id) => [...slotMainStats(tables, id)])),
      sub: new Set(slots.flatMap((id) => [...slotSubStats(tables, id)])),
    }
  }, [filters.slot, tables])

  const options = useMemo(() => {
    const setIds = [...new Set(relics.map((r) => r.setId))]
    return {
      slot: RELIC_SLOTS.map((s) => ({ value: s.id, label: s.name })),
      set: setIds
        .map((id) => ({
          value: id,
          label: tables.relicSets[id]?.name ?? id,
          icon: tables.relicSets[id] && <img className="inv-set-icon" src={resUrl(tables.relicSets[id].icon)} alt="" />,
        }))
        .sort((a, b) => a.label.localeCompare(b.label, 'ru')),
      main: statOptions(relics.flatMap((r) => r.main ?? []).filter((st) => possible.main.has(st.property))),
      sub: statOptions(relics.flatMap((r) => r.subs).filter((st) => possible.sub.has(st.property))),
      status: STATUS,
    }
  }, [relics, tables, possible])

  const groups = (
    [
      ['slot', 'Слот', 'Слот'],
      ['set', 'Набор', ''],
      ['main', 'Основная', 'Осн.'],
      ['sub', 'Доп. характеристики', 'Доп.'],
      ['status', 'Состояние', ''],
    ] as const
  ).map(([key, label, tag]) => ({
    key,
    label,
    tag,
    options: options[key],
    value: filters[key],
    onChange: (v: FilterState) => setFilters((f) => (key === 'slot' ? dropImpossible({ ...f, slot: v }) : { ...f, [key]: v })),
  }))

  // a slot change drops chosen stats that slot can't have, so no tag filters out everything
  function dropImpossible(f: Filters): Filters {
    const slots = RELIC_SLOTS.map((x) => x.id).filter((id) => passOne(f.slot, id))
    const main = new Set(slots.flatMap((id) => [...slotMainStats(tables, id)]))
    const sub = new Set(slots.flatMap((id) => [...slotSubStats(tables, id)]))
    const keep = (state: FilterState, ok: Set<string>) => Object.fromEntries(Object.entries(state).filter(([k]) => ok.has(k)))
    return { ...f, main: keep(f.main, main), sub: keep(f.sub, sub) }
  }

  const noteOf = (uid: number) => notes?.relics[uid]?.text ?? ''
  const isNew = (r: RelicView) => isNewRelic(r.relic, inventory, now)
  const newCount = relics.filter(isNew).length
  const statusOf = (r: RelicView) =>
    [
      isNew(r) && 'new',
      r.relic.locked && 'locked',
      r.relic.discarded && 'discarded',
      r.relic.equippedBy != null && 'equipped',
      noteOf(r.relic.uid) && 'note',
    ].filter((s): s is string => !!s)
  const q = query.trim().toLowerCase()

  const shown = relics
    .filter(
      (r) =>
        passOne(filters.slot, r.slot) &&
        passOne(filters.set, r.setId) &&
        passOne(filters.main, r.main?.property ?? '') &&
        passAll(filters.sub, r.subs.map((s) => s.property)) &&
        passAll(filters.status, statusOf(r)),
    )
    .filter((r) => !q || noteOf(r.relic.uid).toLowerCase().includes(q) || r.setName.toLowerCase().includes(q))
    .sort((a, b) => {
      if (sort === 'new') return b.relic.firstSeen - a.relic.firstSeen || b.relic.uid - a.relic.uid
      if (sort === 'set') return a.setName.localeCompare(b.setName, 'ru') || slotIndex(a) - slotIndex(b)
      return b.rarity - a.rarity || b.relic.level - a.relic.level || b.relic.uid - a.relic.uid
    })

  const selected = open != null ? relics.find((r) => r.relic.uid === open) : undefined
  const wearer = (r: RelicView) =>
    r.relic.equippedBy != null ? characterName(tables, r.relic.equippedBy, profile.nickname) : null

  return (
    <>
      <div className="inv-controls">
        <div className="inv-toolbar">
          <SearchInput value={query} onChange={setQuery} placeholder="Поиск по заметкам и наборам" />
          <Select title="Порядок" value={sort} options={SORTS} onChange={setSort} />
          <span className="inv-toolbar__count">
            {newCount > 0 && (
              <button
                type="button"
                className="inv-new-count"
                title="Показать только новые"
                onClick={() => setFilters((f) => ({ ...f, status: { ...f.status, new: 'include' } }))}
              >
                {newCount} {plural(newCount, 'новая', 'новые', 'новых')}
              </button>
            )}
            {shown.length} из {relics.length}
          </span>
        </div>
        <div className="inv-toolbar">
          {groups.map((g) => (
            <FilterMenu key={g.key} label={g.label} options={g.options} value={g.value} onChange={g.onChange} />
          ))}
        </div>
        <FilterTags groups={groups} onReset={() => setFilters(NO_FILTERS)} />
      </div>

      <Grid min={280}>
        {shown.map((r) => (
          <RelicCard
            key={r.relic.uid}
            relic={r}
            wearer={wearer(r)}
            note={noteOf(r.relic.uid)}
            isNew={isNew(r)}
            onOpen={() => setOpen(r.relic.uid)}
          />
        ))}
      </Grid>

      {selected && (
        <RelicModal
          key={selected.relic.uid}
          relic={selected}
          tables={tables}
          wearer={wearer(selected)}
          note={noteOf(selected.relic.uid)}
          onClose={() => setOpen(null)}
        />
      )}
    </>
  )
}

/** Russian plural: 1 новая, 2 новые, 5 новых. */
function plural(n: number, one: string, few: string, many: string): string {
  const d = n % 10
  const dd = n % 100
  if (d === 1 && dd !== 11) return one
  if (d >= 2 && d <= 4 && (dd < 12 || dd > 14)) return few
  return many
}

const slotIndex = (r: RelicView) => RELIC_SLOTS.findIndex((s) => s.id === r.slot)
function RelicCard({
  relic,
  wearer,
  note,
  isNew,
  onOpen,
}: {
  relic: RelicView
  wearer: string | null
  note: string
  isNew: boolean
  onOpen: () => void
}) {
  return (
    <ItemCard
      image={resUrl(relic.icon)}
      rarity={relic.rarity}
      title={relic.setName}
      meta={`${slotName(relic.slot)} · +${relic.relic.level}`}
      badges={<RelicBadges relic={relic} />}
      isNew={isNew}
      onClick={onOpen}
      footer={
        <>
          <RelicWearer relic={relic} name={wearer} />
          <span className={note ? 'inv-note-mark inv-note-mark--set' : 'inv-note-mark'} title={note ? 'Есть заметка' : 'Добавить заметку'}>
            <NotebookPen size={14} />
          </span>
        </>
      }
    >
      <RelicStats relic={relic} />
      {note && <p className="inv-note-text">{note}</p>}
    </ItemCard>
  )
}
