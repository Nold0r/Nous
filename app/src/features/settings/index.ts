import { defineFeature } from '../types'
import { SettingsPage } from './SettingsPage'

export default defineFeature({
  id: 'settings',
  title: 'Настройки',
  icon: 'settings',
  category: 'system',
  component: SettingsPage,
})
