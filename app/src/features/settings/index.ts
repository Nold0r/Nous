import { defineScreen } from '../types'
import { SettingsPage } from './SettingsPage'

export default defineScreen({
  id: 'settings',
  title: 'Настройки',
  icon: 'settings',
  category: 'system',
  component: SettingsPage,
})
