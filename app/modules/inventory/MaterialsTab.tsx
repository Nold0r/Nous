import { Panel } from '@app/ui'
import type { Inventory, Material } from '@app/lib/inventory'
import { resUrl, type GameTables } from '@app/lib/game-tables'
import { formatNumber } from '@app/lib/format'

// Relic upgrades: exp materials (RelicExp) plus relic remains, Self-Modeling
// and Miracle resin, Variable Dice and Interference Key (ItemConfig ids).
const RELIC_ITEMS = new Set([235, 236, 237, 238, 239])
const GROWTH = new Set(['AvatarExp', 'EquipmentExp', 'TracePath', 'CommonMonsterDrop', 'WeeklyMonsterDrop', 'AvatarRank', 'Eidolon'])

const GROUPS = [
  { title: 'Улучшение реликвий', has: (tid: number, sub?: string) => RELIC_ITEMS.has(tid) || sub === 'RelicExp' },
  { title: 'Развитие персонажей и конусов', has: (_: number, sub?: string) => !!sub && GROWTH.has(sub) },
  { title: 'Прочее', has: () => true },
]

export function MaterialsTab({ inventory, tables }: { inventory: Inventory; tables: GameTables }) {
  const groups = GROUPS.map((g) => ({ title: g.title, items: [] as Material[] }))
  for (const m of Object.values(inventory.materials)) {
    if (m.count <= 0) continue
    const sub = tables.items[m.tid]?.subType
    groups[GROUPS.findIndex((g) => g.has(m.tid, sub))].items.push(m)
  }
  const rarity = (m: Material) => tables.items[m.tid]?.rarity ?? 1

  return (
    <>
      {groups
        .filter((g) => g.items.length > 0)
        .map((g) => (
          <Panel key={g.title} title={g.title} aside={g.items.length}>
            <div className="inv-items">
              {g.items
                .sort((a, b) => rarity(b) - rarity(a) || a.tid - b.tid)
                .map((m) => {
                  const def = tables.items[m.tid]
                  const name = def?.name ?? `Предмет ${m.tid}`
                  return (
                    <div key={m.tid} className={`inv-item ui-card__tile--r${def?.rarity ?? 1}`} title={name}>
                      <img src={resUrl(def?.icon ?? `icon/item/${m.tid}.png`)} alt="" draggable={false} />
                      <span className="inv-item__count">{formatNumber(m.count, { compact: true })}</span>
                      <span className="inv-item__name">{name}</span>
                    </div>
                  )
                })}
            </div>
          </Panel>
        ))}
    </>
  )
}
