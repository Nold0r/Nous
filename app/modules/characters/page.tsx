import { Route, Routes } from 'react-router-dom'
import { EmptyState, Page } from '@app/ui'
import { useInventory } from '@app/lib/inventory'
import { useCharacterTables, useGameTables } from '@app/lib/game-tables'
import { CharacterList } from './CharacterList'
import { CharacterDetail } from './CharacterDetail'
import './characters.css'

/** List at /characters, one character at /characters/<path avatar id>. */
export function CharactersPage() {
  const inventory = useInventory()
  const tables = useGameTables()
  const characterTables = useCharacterTables()
  const empty = !inventory || Object.keys(inventory.avatarPaths).length === 0

  if (empty || !tables || !characterTables) {
    return (
      <Page title="Персонажи" subtitle="Ваши персонажи и их снаряжение">
        <EmptyState icon="characters" title={empty ? 'Персонажи не загружены' : 'Загрузка…'}>
          {empty && 'Войдите в игру с запущенным Nous, чтобы он получил список ваших персонажей.'}
        </EmptyState>
      </Page>
    )
  }
  return (
    <Routes>
      <Route index element={<CharacterList inventory={inventory} tables={tables} characterTables={characterTables} />} />
      <Route
        path=":id"
        element={<CharacterDetail inventory={inventory} tables={tables} characterTables={characterTables} />}
      />
    </Routes>
  )
}

export default CharactersPage
