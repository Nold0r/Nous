import { EmptyState, Page } from '@app/ui'

export function RelicsPage() {
  return (
    <Page title="Реликвии" subtitle="Инвентарь реликвий">
      <EmptyState icon="inventory" title="Реликвии не загружены">
        Войдите в игру, чтобы Nous получил ваш инвентарь реликвий.
      </EmptyState>
    </Page>
  )
}

export default RelicsPage
