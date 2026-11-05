import { useState } from 'react'
import { Lock } from 'lucide-react'
import { Grid, ItemCard, Select } from '@app/ui'
import type { Inventory } from '@app/lib/inventory'
import { useProfile } from '@app/lib/profile'
import { characterIcon, characterName, resUrl, type GameTables } from '@app/lib/game-tables'

const RARITIES = [
  { value: 0, label: 'Любая редкость' },
  { value: 5, label: '5★' },
  { value: 4, label: '4★' },
  { value: 3, label: '3★' },
] as const

export function LightConesTab({ inventory, tables }: { inventory: Inventory; tables: GameTables }) {
  const profile = useProfile()
  const [rarity, setRarity] = useState<number>(0)

  const all = Object.values(inventory.lightCones).map((c) => ({ cone: c, def: tables.lightCones[c.tid] }))
  const shown = all
    .filter((c) => !rarity || c.def?.rarity === rarity)
    .sort((a, b) => (b.def?.rarity ?? 0) - (a.def?.rarity ?? 0) || b.cone.level - a.cone.level || a.cone.tid - b.cone.tid)

  return (
    <>
      <div className="inv-toolbar">
        <Select title="Редкость" value={rarity} options={RARITIES} onChange={setRarity} />
        <span className="inv-toolbar__count">
          {shown.length} из {all.length}
        </span>
      </div>
      <Grid min={260}>
        {shown.map(({ cone, def }) => (
          <ItemCard
            key={cone.uid}
            image={resUrl(def?.icon ?? `icon/light_cone/${cone.tid}.png`)}
            rarity={def?.rarity ?? 3}
            title={def?.name ?? `Конус ${cone.tid}`}
            meta={`Ур. ${cone.level} · Наложение ${cone.superimposition}`}
            badges={cone.locked && <Lock size={14} aria-label="Заблокирован" />}
            footer={
              cone.equippedBy != null ? (
                <span className="inv-wearer">
                  <img src={characterIcon(cone.equippedBy)} alt="" loading="lazy" />
                  {characterName(tables, cone.equippedBy, profile.nickname)}
                </span>
              ) : (
                <span className="text-dim">Не надет</span>
              )
            }
          />
        ))}
      </Grid>
    </>
  )
}
