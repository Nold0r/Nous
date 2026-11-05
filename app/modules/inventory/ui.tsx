import { useState } from 'react'
import { EmptyState, Page, Tabs } from '@app/ui'
import { useInventory } from '@app/lib/inventory'
import { useGameTables } from '@app/lib/game-tables'
import { RelicsTab } from './RelicsTab'
import { LightConesTab } from './LightConesTab'
import { MaterialsTab } from './MaterialsTab'
import './inventory.css'

const TABS = [
  { id: 'relics', label: 'Реликвии' },
  { id: 'light-cones', label: 'Световые конусы' },
  { id: 'materials', label: 'Материалы' },
] as const

type Tab = (typeof TABS)[number]['id']

const TAB_KEY = 'inventory.tab'

function savedTab(): Tab {
  try {
    const v = localStorage.getItem(TAB_KEY)
    return TABS.some((t) => t.id === v) ? (v as Tab) : 'relics'
  } catch {
    return 'relics'
  }
}

export function InventoryPage() {
  const inventory = useInventory()
  const tables = useGameTables()
  const [tab, setTab] = useState<Tab>(savedTab)

  const choose = (id: Tab) => {
    setTab(id)
    try {
      localStorage.setItem(TAB_KEY, id)
    } catch {
      // the tab just won't be remembered
    }
  }

  const empty = !inventory || (inventory.syncedAt == null && Object.keys(inventory.relics).length === 0)

  return (
    <Page title="Инвентарь" subtitle="Реликвии, световые конусы и материалы" actions={<Tabs items={TABS} value={tab} onChange={choose} />}>
      {empty || !tables ? (
        <EmptyState icon="inventory" title={empty ? 'Инвентарь не загружен' : 'Загрузка…'}>
          {empty && 'Войдите в игру с запущенным Nous, чтобы он получил ваш инвентарь.'}
        </EmptyState>
      ) : tab === 'relics' ? (
        <RelicsTab inventory={inventory} tables={tables} />
      ) : tab === 'light-cones' ? (
        <LightConesTab inventory={inventory} tables={tables} />
      ) : (
        <MaterialsTab inventory={inventory} tables={tables} />
      )}
    </Page>
  )
}
export default InventoryPage
