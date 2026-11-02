import { defineModule } from '../types'
import { OverviewPage } from './OverviewPage'

export default defineModule({
  id: 'overview',
  title: 'Обзор',
  requires: ['hsr'],
  screens: [{
    id: 'overview',
    title: 'Обзор',
    icon: 'overview',
    category: 'main',
    isDefault: true,
    component: OverviewPage,
  }],
})
