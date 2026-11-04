import { EmptyState, Page } from '@app/ui'

export function CharactersPage() {
  return (
    <Page title="Персонажи" subtitle="Ваши персонажи и их снаряжение">
      <EmptyState icon="characters" title="Персонажи не загружены">
        Войдите в игру, чтобы Nous получил список ваших персонажей.
      </EmptyState>
    </Page>
  )
}

export default CharactersPage
