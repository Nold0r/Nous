import { EmptyState, Page } from '../../ui'

export function MapPage() {
  return (
    <Page title="Карта" subtitle="Текущая локация">
      <EmptyState icon="map" title="Карта появится позже">
        Здесь будет карта локации с позицией игрока и сундуками.
      </EmptyState>
    </Page>
  )
}
