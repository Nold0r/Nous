import { defineFeature } from '../types'
import { RelicsPage } from './RelicsPage'

export default defineFeature({
  id: 'relics',
  title: 'Реликвии',
  icon: 'inventory',
  category: 'inventory',
  order: 1,
  component: RelicsPage,
})
