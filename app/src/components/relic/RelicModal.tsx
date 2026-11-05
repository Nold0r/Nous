import { useState } from 'react'
import { Lock, NotebookPen, Trash2 } from 'lucide-react'
import { Badge, Button, Modal, Pager, Rarity, SectionTitle, SetEffects, StatLine, TextArea } from '../../ui'
import { setRelicNote } from '../../lib/inventory'
import { formatStat, resUrl, slotName, type GameTables, type RelicView } from '../../lib/game-tables'
import { RelicStats, RelicWearer } from './RelicParts'

/**
 * A relic in detail with its note, used by the inventory and the characters
 * screens. With `pager` the dialog pages through a list (a character's six
 * relics): arrows in the head, ←/→ on the keyboard.
 */
export function RelicModal({
  relic,
  tables,
  wearer,
  note,
  pager,
  onClose,
}: {
  relic: RelicView
  tables: GameTables
  wearer: string | null
  note: string
  pager?: { index: number; count: number; onChange: (index: number) => void }
  onClose: () => void
}) {
  const [text, setText] = useState(note)
  const [shownUid, setShownUid] = useState(relic.relic.uid)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // paging to another relic: start from its own note
  if (shownUid !== relic.relic.uid) {
    setShownUid(relic.relic.uid)
    setText(note)
    setError(null)
  }
  const changed = text.trim() !== note

  const save = (value: string, close: boolean) => {
    if (saving) return
    setSaving(true)
    setError(null)
    setRelicNote(relic.relic.uid, value)
      .then(() => (close ? onClose() : setText(value.trim())))
      .catch((e) => setError(String(e)))
      .finally(() => setSaving(false))
  }

  const desc = tables.relicSets[relic.setId]?.desc ?? []
  const pieces = relic.slot === 'NECK' || relic.slot === 'OBJECT' ? [2] : [2, 4]
  const stars = Math.min(5, Math.max(1, relic.rarity)) as 1 | 2 | 3 | 4 | 5

  return (
    <Modal
      open
      size="lg"
      title={relic.setName}
      subtitle={`${slotName(relic.slot)} · +${relic.relic.level}`}
      actions={pager && <Pager {...pager} />}
      onClose={onClose}
      footer={
        <>
          {note && (
            <Button variant="danger" className="relic-modal__delete" disabled={saving} onClick={() => save('', !pager)}>
              Удалить заметку
            </Button>
          )}
          <Button variant="secondary" onClick={onClose}>
            {pager ? 'Закрыть' : 'Отмена'}
          </Button>
          <Button variant="primary" disabled={saving || !changed} onClick={() => save(text, !pager)}>
            Сохранить
          </Button>
        </>
      }
    >
      <div className="relic-modal">
        <section className="relic-modal__relic">
          <div className={`relic-modal__hero ui-card--r${stars}`}>
            <span className={`relic-modal__tile ui-card__tile ui-card__tile--r${stars}`}>
              <img src={resUrl(relic.icon)} alt="" />
            </span>
            <div className="relic-modal__facts">
              <Rarity stars={stars} />
              <RelicWearer relic={relic} name={wearer} />
              {(relic.relic.locked || relic.relic.discarded) && (
                <span className="relic-modal__flags">
                  {relic.relic.locked && (
                    <Badge>
                      <Lock size={11} /> Заблокирована
                    </Badge>
                  )}
                  {relic.relic.discarded && (
                    <Badge tone="danger">
                      <Trash2 size={11} /> На разбор
                    </Badge>
                  )}
                </span>
              )}
            </div>
          </div>
          <RelicStats relic={relic} labels />
          {relic.reforge.length > 0 && (
            <div className="relic-modal__reforge">
              <Badge tone="warning">Переброс не принят</Badge>
              {relic.reforge.map((s) => (
                <StatLine key={s.property} icon={s.icon} label={s.name} value={formatStat(s)} rolls={s.rolls ? s.rolls - 1 : 0} showLabel />
              ))}
            </div>
          )}
        </section>

        <section className="relic-modal__side">
          <SectionTitle>Бонусы набора</SectionTitle>
          <SetEffects sets={[{ name: relic.setName, bonuses: desc.map((d, i) => ({ pieces: pieces[i], text: d })) }]} />

          <SectionTitle aside={<NotebookPen size={14} />}>Заметка</SectionTitle>
          <TextArea
            autoFocus={!pager}
            rows={5}
            maxLength={2000}
            placeholder="Например: на Ахерон, не продавать; переброс на крит"
            hint="Ctrl+Enter — сохранить. Заметка остаётся у реликвии после прокачки и переброса."
            value={text}
            onChange={setText}
            onSubmit={() => changed && save(text, !pager)}
          />
          {error && <p className="relic-error">{error}</p>}
        </section>
      </div>
    </Modal>
  )
}
