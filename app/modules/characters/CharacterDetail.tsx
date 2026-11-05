import { useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { ChevronLeft, Lock } from 'lucide-react'
import {
  Badge,
  Button,
  EmptyState,
  ItemCard,
  Page,
  Panel,
  Rarity,
  StatIcon,
  StatLine,
  StatList,
  StatRow,
  TextArea,
} from '@app/ui'
import { setCharacterNote, useNotes, type Inventory } from '@app/lib/inventory'
import { useProfile } from '@app/lib/profile'
import { RELIC_SLOTS, formatStat, resUrl, type CharacterTables, type GameTables, type RelicView } from '@app/lib/game-tables'
import { formatTotal, maxLevelOf, viewCharacter, type CharacterView } from '@app/lib/characters'

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
  return (
    <Page title={c.name} subtitle={`${c.elementName} · ${c.pathName}`} actions={back}>
      <div className="char-detail">
        <section className="char-detail__hero">
          <div className={`char-portrait char-portrait--r${stars}`}>
            <img src={resUrl(ct.characterIcons[c.id]?.portrait ?? `image/character_portrait/${c.id}.png`)} alt="" />
            <div className="char-portrait__info">
              <Rarity stars={stars} />
              <span className="char-portrait__level">
                Ур. {c.avatar.level}
                <span className="text-dim">/{maxLevelOf(c.avatar.ascension)}</span>
              </span>
              {!c.isCurrentPath && <Badge>Путь не выбран в игре</Badge>}
            </div>
          </div>
          <Eidolons c={c} ct={ct} />
          <CharacterNote id={c.id} />
        </section>

        <section className="char-detail__column">
          <Panel title="Характеристики" aside="как в игре, без условных бонусов">
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
                        <span className="char-stat__bonus">+{formatTotal({ ...s, value: s.value - s.base })}</span>
                      )}
                    </>
                  }
                />
              ))}
            </StatList>
          </Panel>

          {c.skills.length > 0 && (
            <Panel title="Умения">
              <div className="char-skills">
                {c.skills.map((s) => (
                  <div key={s.anchor} className="char-skill" title={s.name}>
                    <img src={resUrl(s.icon)} alt="" />
                    <span className="char-skill__text">
                      <span className="char-skill__type">{s.typeText}</span>
                      <span className="char-skill__name">{s.name}</span>
                    </span>
                    <span className="char-skill__level">
                      {s.level + s.bonus}
                      {s.bonus > 0 && <span className="char-stat__bonus" title="от эйдолонов">+{s.bonus}</span>}
                      <span className="text-dim">/{s.maxLevel + (s.bonus > 0 ? s.bonus : 0)}</span>
                    </span>
                  </div>
                ))}
              </div>
            </Panel>
          )}

          {c.traceBonuses.length > 0 && (
            <Panel title="Бонусы следов">
              <div className="char-traces">
                {c.traceBonuses.map((p) => {
                  const prop = tables.properties[p.type]
                  return (
                    <span key={p.type} className="char-trace" title={prop?.name}>
                      <StatIcon icon={prop?.icon ?? ''} size={16} />
                      {prop?.name ?? p.type}
                      <b>+{formatStat({ property: p.type, name: '', icon: '', value: p.value, percent: prop?.percent ?? false })}</b>
                    </span>
                  )
                })}
              </div>
            </Panel>
          )}
        </section>

        <section className="char-detail__column">
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
            <div className="char-empty-slot">Световой конус не надет</div>
          )}

          <Panel
            title="Реликвии"
            aside={c.sets.flatMap((s) => (s.active.length ? [`${s.name} ×${s.count}`] : [])).join(' · ') || undefined}
          >
            <div className="char-relics">
              {c.relics.map((r, i) => (r ? <RelicMini key={i} relic={r} /> : <div key={i} className="char-empty-slot">{RELIC_SLOTS[i].name}: пусто</div>))}
            </div>
            {c.sets.some((s) => s.active.length) && (
              <div className="char-sets">
                {c.sets
                  .filter((s) => s.active.length)
                  .map((s) => (
                    <div key={s.setId} className="char-set">
                      <img src={resUrl(s.icon)} alt="" />
                      <span className="char-set__name">{s.name}</span>
                      {s.active.map((n) => (
                        <Badge key={n} tone="gold">
                          {n} предмета
                        </Badge>
                      ))}
                    </div>
                  ))}
              </div>
            )}
          </Panel>
        </section>
      </div>
    </Page>
  )
}

function Eidolons({ c, ct }: { c: CharacterView; ct: CharacterTables }) {
  const list = ct.eidolons[c.id] ?? []
  return (
    <div className="char-eidolons" aria-label={`Эйдолоны: ${c.build.eidolon} из 6`}>
      {list.map((e, i) => (
        <span
          key={i}
          className={`char-eidolons__item${i < c.build.eidolon ? ' char-eidolons__item--on' : ''}`}
          title={`E${i + 1}. ${e.name}\n\n${e.desc}`}
        >
          <img src={resUrl(e.icon)} alt="" />
          <span>E{i + 1}</span>
        </span>
      ))}
    </div>
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
    <div className="char-note">
      <TextArea
        label="Заметка"
        rows={4}
        maxLength={2000}
        placeholder="Например: нужен E2 и сфера на электро; фармить «Вор падающего метеора»"
        hint="Ctrl+Enter — сохранить"
        value={value}
        onChange={setText}
        onSubmit={() => changed && save(value)}
      />
      {(changed || saved) && (
        <div className="char-note__actions">
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

function RelicMini({ relic }: { relic: RelicView }) {
  return (
    <ItemCard
      image={resUrl(relic.icon)}
      rarity={relic.rarity}
      title={relic.setName}
      meta={`+${relic.relic.level}`}
      badges={relic.relic.locked && <Lock size={12} aria-label="Заблокирована" />}
    >
      {relic.main && <StatLine icon={relic.main.icon} label={relic.main.name} value={formatStat(relic.main)} main showLabel />}
      <div className="char-relic-subs">
        {relic.subs.map((s) => (
          <StatLine key={s.property} icon={s.icon} label={s.name} value={formatStat(s)} rolls={s.rolls ? s.rolls - 1 : 0} />
        ))}
      </div>
    </ItemCard>
  )
}
