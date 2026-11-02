import { defineFeature } from '../types'
import { OverviewPage } from './OverviewPage'

export default defineFeature({
  id: 'overview',
  title: 'Обзор',
  icon: 'overview',
  category: 'main',
  isDefault: true,
  component: OverviewPage,
})
