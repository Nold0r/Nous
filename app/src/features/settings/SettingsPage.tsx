import { EmptyState, Page } from '../../ui'

export function SettingsPage() {
  return (
    <Page title="Настройки">
      <EmptyState icon="settings" title="Настроек пока нет">
        Здесь появятся настройки приложения.
      </EmptyState>
    </Page>
  )
}
