import { EmptyState, Page } from '../../ui'

export function OverviewPage() {
  return (
    <Page title="Обзор" subtitle="Сводка по аккаунту">
      <EmptyState icon="overview" title="Сводка появится позже">
        Здесь будут энергия освоения, ежедневные задания и прогресс исследования.
      </EmptyState>
    </Page>
  )
}
