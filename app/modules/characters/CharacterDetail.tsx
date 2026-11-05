import { useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { ChevronLeft, Lock } from 'lucide-react'
import {
  Badge,
  Button,
  EmptyState,
  HeroArt,
  ItemCard,
  Page,
  Panel,
  RankRow,
  Rarity,
  RelicOrbit,
  SetEffects,
  SkillRow,
  StatChip,
  StatIcon,
  StatList,
  StatRow,
  TextArea,
} from '@app/ui'
import { RelicModal } from '@app/components/relic'
import { SpineArt } from '@app/components/spine/SpineArt'
import { setCharacterNote, useNotes, type Inventory } from '@app/lib/inventory'
import { useProfile } from '@app/lib/profile'
import { RELIC_SLOTS, formatStat, resUrl, type CharacterTables, type GameTables } from '@app/lib/game-tables'
import { formatTotal, maxLevelOf, viewCharacter } from '@app/lib/characters'

export function CharacterDetail({
  inventory,
  tables,
  characterTables: ct,
}: {
  inventory: Inventory
  tables: GameTables
  characterTables: CharacterTables
}) {
  const navigate = useNavigate()
  const { id } = useParams()
  const profile = useProfile()
  const notes = useNotes()
  const [relicIndex, setRelicIndex] = useState<number | null>(null)
  const c = useMemo(
    () => viewCharacter(tables, ct, inventory, Number(id), profile.nickname),
    [tables, ct, inventory, id, profile.nickname],
  )
  const back = (
    <Button variant="ghost" icon={ChevronLeft} onClick={() => navigate('..')}>
      Все персонажи
    </Button>
  )

  if (!c) {
    return (
      <Page title="Персонаж" actions={back}>
        <EmptyState icon="characters" title="Персонаж не найден">
          Его нет в данных аккаунта или в таблицах игры (новый персонаж: обновите таблицы).
        </EmptyState>
      </Page>
    )
  }

  const stars = Math.min(5, Math.max(1, c.rarity)) as 1 | 2 | 3 | 4 | 5
  // the dialog pages through the relics actually worn
  const worn = c.relics.flatMap((r) => (r ? [r] : []))
  const shown = relicIndex != null ? worn[relicIndex] : undefined

  return (
    <Page title={c.name} subtitle={`${c.elementName} · ${c.pathName}`} actions={back}>
      <div className="char-page">
        <section className="char-page__hero">
          <HeroArt
            rarity={stars}
            art={
              <SpineArt
                characterId={c.id}
                fallback={resUrl(ct.characterIcons[c.id]?.portrait ?? `image/character_portrait/${c.id}.png`)}
              />
            }
          >
            <Rarity stars={stars} />
            <span className="char-page__level">
              Ур. {c.avatar.level}
              <span className="text-dim">/{maxLevelOf(c.avatar.ascension)}</span>
            </span>
            <span className="char-page__tags">
              {ct.elements[c.element] && (
                <Badge>
                  <img className="char-tag-icon" src={resUrl(ct.elements[c.element].icon)} alt="" />
                  {c.elementName}
                </Badge>
              )}
              {ct.paths[c.path] && (
                <Badge>
                  <img className="char-tag-icon" src={resUrl(ct.paths[c.path].icon)} alt="" />
                  {c.pathName}
                </Badge>
              )}
              {!c.isCurrentPath && <Badge tone="warning">Путь не выбран в игре</Badge>}
            </span>
          </HeroArt>
          <RankRow
            items={(ct.eidolons[c.id] ?? []).map((e, i) => ({
              image: resUrl(e.icon),
              label: `E${i + 1}`,
              active: i < c.build.eidolon,
              title: `E${i + 1}. ${e.name}\n\n${e.desc}`,
            }))}
          />
        </section>

        <section className="char-page__side">
          <Panel title="Характеристики" aside="без условных бонусов">
            <StatList>
              {c.stats.map((s) => (
                <StatRow
                  key={s.key}
                  icon={<StatIcon icon={s.icon} size={22} />}
                  label={s.name}
                  hint={
                    s.base != null
                      ? `${formatTotal(s, true)} = база ${formatTotal({ ...s, value: s.base }, true)} + бонус ${formatTotal({ ...s, value: s.value - s.base }, true)}`
                      : undefined
                  }
                  value={
                    <>
                      {formatTotal(s)}
                      {s.base != null && s.value - s.base >= 0.5 && (
                        <span className="char-stat-bonus">+{formatTotal({ ...s, value: s.value - s.base })}</span>
                      )}
                    </>
                  }
                />
              ))}
            </StatList>
          </Panel>

          {c.lightCone ? (
            <ItemCard
              image={resUrl(c.lightCone.icon)}
              rarity={c.lightCone.rarity}
              title={c.lightCone.name}
              meta={`Ур. ${c.lightCone.cone.level} · Наложение ${c.lightCone.cone.superimposition}`}
              badges={c.lightCone.cone.locked && <Lock size={14} aria-label="Заблокирован" />}
              footer={c.lightCone.skill && <span className="text-gold">{c.lightCone.skill}</span>}
            />
          ) : (
            <EmptyState title="Световой конус не надет" />
          )}
        </section>
      </div>

      <Panel title="Реликвии" aside="нажмите на реликвию, чтобы увидеть характеристики">
        <div className="char-relics">
          <RelicOrbit
            slots={c.relics.map((r) =>
              r ? { image: resUrl(r.icon), rarity: r.rarity, level: `+${r.relic.level}`, label: `${r.setName}, +${r.relic.level}` } : null,
            )}
            emptyLabels={RELIC_SLOTS.map((s) => s.name)}
            onSelect={(slot) => setRelicIndex(worn.indexOf(c.relics[slot]!))}
          />
          {c.sets.length > 0 ? (
            <SetEffects
              sets={c.sets.map((s) => {
                const desc = tables.relicSets[s.setId]?.desc ?? []
                const planar = desc.length === 1
                return {
                  name: s.name,
                  icon: s.icon ? resUrl(s.icon) : undefined,
                  count: s.count,
                  bonuses: desc.map((text, i) => ({ pieces: planar ? 2 : (i + 1) * 2, text })),
                }
              })}
            />
          ) : (
            <EmptyState title="Реликвии не надеты" />
          )}
        </div>
      </Panel>

      <div className="char-page__bottom">
        {c.skills.length > 0 && (
          <Panel title="Умения">
            <div className="char-stack">
              {c.skills.map((s) => (
                <SkillRow key={s.anchor} icon={resUrl(s.icon)} kind={s.typeText} name={s.name} level={s.level} bonus={s.bonus} max={s.maxLevel} />
              ))}
            </div>
          </Panel>
        )}

        <div className="char-stack">
          {c.traceBonuses.length > 0 && (
            <Panel title="Бонусы следов">
              <div className="char-chips">
                {c.traceBonuses.map((p) => {
                  const prop = tables.properties[p.type]
                  const value = formatStat({ property: p.type, name: '', icon: '', value: p.value, percent: prop?.percent ?? false })
                  return <StatChip key={p.type} icon={prop?.icon ?? ''} label={prop?.name ?? p.type} value={`+${value}`} />
                })}
              </div>
            </Panel>
          )}
          <Panel title="Заметка">
            <CharacterNote id={c.id} />
          </Panel>
        </div>
      </div>

      {shown && (
        <RelicModal
          relic={shown}
          tables={tables}
          wearer={c.name}
          note={notes?.relics[shown.relic.uid]?.text ?? ''}
          pager={{ index: relicIndex!, count: worn.length, onChange: setRelicIndex }}
          onClose={() => setRelicIndex(null)}
        />
      )}
    </Page>
  )
}

function CharacterNote({ id }: { id: number }) {
  const notes = useNotes()
  const saved = notes?.characters?.[id]?.text ?? ''
  const [text, setText] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const value = text ?? saved
  const changed = value.trim() !== saved

  const save = (v: string) => {
    setError(null)
    setCharacterNote(id, v)
      .then(() => setText(null))
      .catch((e) => setError(String(e)))
  }

  return (
    <div className="char-stack">
      <TextArea
        rows={4}
        maxLength={2000}
        placeholder="Например: нужен E2 и сфера на электро; фармить «Вор падающего метеора»"
        hint="Ctrl+Enter — сохранить"
        value={value}
        onChange={setText}
        onSubmit={() => changed && save(value)}
      />
      {(changed || saved) && (
        <div className="char-note-actions">
          {saved && (
            <Button variant="danger" size="sm" onClick={() => save('')}>
              Удалить
            </Button>
          )}
          {changed && (
            <>
              <Button size="sm" variant="ghost" onClick={() => setText(null)}>
                Отменить
              </Button>
              <Button size="sm" variant="primary" onClick={() => save(value)}>
                Сохранить
              </Button>
            </>
          )}
        </div>
      )}
      {error && <p className="char-error">{error}</p>}
    </div>
  )
}

