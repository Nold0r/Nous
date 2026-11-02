import { defineModule } from '../types'
import { RelicsPage } from './RelicsPage'

export default defineModule({
  id: 'relics',
  title: 'Реликвии',
  requires: ['hsr'],
  screens: [{
    id: 'relics',
    title: 'Реликвии',
    icon: 'inventory',
    category: 'inventory',
    order: 1,
    component: RelicsPage,
  }],
})
