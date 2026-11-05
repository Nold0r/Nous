import { useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { NotebookPen } from 'lucide-react'
import {
  Badge,
  FilterMenu,
  FilterTags,
  Grid,
  Page,
  PortraitCard,
  SearchInput,
  Select,
  type FilterOption,
  type FilterState,
} from '@app/ui'
import { useNotes, type Inventory } from '@app/lib/inventory'
import { useProfile } from '@app/lib/profile'
import { resUrl, type CharacterTables, type GameTables } from '@app/lib/game-tables'
import { shownPaths, viewCharacter, type CharacterView } from '@app/lib/characters'

type Sort = 'level' | 'rarity' | 'eidolon' | 'name'

const SORTS = [
  { value: 'level', label: 'По уровню' },
  { value: 'rarity', label: 'По редкости' },
  { value: 'eidolon', label: 'По эйдолонам' },
  { value: 'name', label: 'По имени' },
] as const

const STATUS: FilterOption[] = [
  { value: 'note', label: 'С заметкой' },
  { value: 'no-cone', label: 'Без светового конуса' },
  { value: 'few-relics', label: 'Не все реликвии надеты' },
]

const RARITY: FilterOption[] = [
  { value: '5', label: '5★' },
  { value: '4', label: '4★' },
]

type Filters = Record<'element' | 'path' | 'rarity' | 'status', FilterState>
const NO_FILTERS: Filters = { element: {}, path: {}, rarity: {}, status: {} }

/** One value per character: white list and black list. */
function passOne(f: FilterState, value: string): boolean {
  if (f[value] === 'exclude') return false
  return !Object.values(f).includes('include') || f[value] === 'include'
}

/** Several flags per character: has every included one, none of the excluded. */
function passAll(f: FilterState, values: string[]): boolean {
  return Object.entries(f).every(([v, mode]) => values.includes(v) === (mode === 'include'))
}

export function CharacterList({
  inventory,
  tables,
  characterTables: ct,
}: {
  inventory: Inventory
  tables: GameTables
  characterTables: CharacterTables
}) {
  const navigate = useNavigate()
  const notes = useNotes()
  const profile = useProfile()
  const [filters, setFilters] = useState<Filters>(NO_FILTERS)
  const [sort, setSort] = useState<Sort>('level')
  const [query, setQuery] = useState('')

  const characters = useMemo(
    () => shownPaths(inventory).flatMap((id) => viewCharacter(tables, ct, inventory, id, profile.nickname) ?? []),
    [inventory, tables, ct, profile.nickname],
  )

  const icon = (src: string) => <img className="char-filter-icon" src={resUrl(src)} alt="" />
  const options = useMemo(() => {
    const present = (key: 'element' | 'path') => [...new Set(characters.map((c) => c[key]))]
    return {
      element: present('element').map((e) => ({
        value: e,
        label: ct.elements[e]?.name ?? e,
        icon: ct.elements[e] && icon(ct.elements[e].icon),
      })),
      path: present('path').map((p) => ({ value: p, label: ct.paths[p]?.name ?? p, icon: ct.paths[p] && icon(ct.paths[p].icon) })),
      rarity: RARITY,
      status: STATUS,
    }
  }, [characters, ct])

  const groups = (
    [
      ['element', 'Стихия', ''],
      ['path', 'Путь', ''],
      ['rarity', 'Редкость', ''],
      ['status', 'Состояние', ''],
    ] as const
  ).map(([key, label, tag]) => ({
    key,
    label,
    tag,
    options: options[key],
    value: filters[key],
    onChange: (v: FilterState) => setFilters((f) => ({ ...f, [key]: v })),
  }))

  const noteOf = (id: number) => notes?.characters?.[id]?.text ?? ''
  const statusOf = (c: CharacterView) =>
    [noteOf(c.id) && 'note', !c.lightCone && 'no-cone', c.relics.some((r) => !r) && 'few-relics'].filter(
      (s): s is string => !!s,
    )
  const q = query.trim().toLowerCase()

  const shown = characters
    .filter(
      (c) =>
        passOne(filters.element, c.element) &&
        passOne(filters.path, c.path) &&
        passOne(filters.rarity, String(c.rarity)) &&
        passAll(filters.status, statusOf(c)),
    )
    .filter((c) => !q || c.name.toLowerCase().includes(q) || noteOf(c.id).toLowerCase().includes(q))
    .sort((a, b) => {
      if (sort === 'name') return a.name.localeCompare(b.name, 'ru')
      if (sort === 'rarity') return b.rarity - a.rarity || b.avatar.level - a.avatar.level
      if (sort === 'eidolon') return b.build.eidolon - a.build.eidolon || b.rarity - a.rarity
      return b.avatar.level - a.avatar.level || b.rarity - a.rarity || a.name.localeCompare(b.name, 'ru')
    })

  return (
    <Page title="Персонажи" subtitle="Ваши персонажи и их снаряжение">
      <div className="char-controls">
        <div className="char-toolbar">
          <SearchInput value={query} onChange={setQuery} placeholder="Поиск по имени и заметкам" />
          <Select title="Порядок" value={sort} options={SORTS} onChange={setSort} />
          <span className="char-toolbar__count">
            {shown.length} из {characters.length}
          </span>
        </div>
        <div className="char-toolbar">
          {groups.map((g) => (
            <FilterMenu key={g.key} label={g.label} options={g.options} value={g.value} onChange={g.onChange} />
          ))}
        </div>
        <FilterTags groups={groups} onReset={() => setFilters(NO_FILTERS)} />
      </div>

      <Grid min={150}>
        {shown.map((c) => (
          <PortraitCard
            key={c.id}
            image={resUrl(ct.characterIcons[c.id]?.icon ?? `icon/character/${c.id}.png`)}
            rarity={c.rarity}
            title={c.name}
            subtitle={
              <>
                {c.pathName}
                {noteOf(c.id) && <NotebookPen size={12} className="char-note-mark" aria-label="Есть заметка" />}
              </>
            }
            dimmed={!c.isCurrentPath}
            topLeft={
              <>
                {ct.elements[c.element] && (
                  <img className="char-badge-icon" src={resUrl(ct.elements[c.element].icon)} alt={c.elementName} title={c.elementName} />
                )}
                {ct.paths[c.path] && (
                  <img className="char-badge-icon char-badge-icon--path" src={resUrl(ct.paths[c.path].icon)} alt={c.pathName} title={c.pathName} />
                )}
              </>
            }
            topRight={
              c.build.eidolon > 0 && (
                <Badge tone="gold" solid>
                  E{c.build.eidolon}
                </Badge>
              )
            }
            bottom={
              <>
                <span>Ур. {c.avatar.level}</span>
                {c.lightCone ? (
                  <span className="char-cone-mark" title={`${c.lightCone.name}, наложение ${c.lightCone.cone.superimposition}`}>
                    <img src={resUrl(c.lightCone.icon)} alt="" />S{c.lightCone.cone.superimposition}
                  </span>
                ) : (
                  <span className="text-dim">без конуса</span>
                )}
              </>
            }
            onClick={() => navigate(String(c.id))}
          />
        ))}
      </Grid>
    </Page>
  )
}
