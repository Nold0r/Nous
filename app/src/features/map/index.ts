import { defineModule } from '../types'
import { MapPage } from './MapPage'

// Map of the current location (placeholder for now).
export default defineModule({
  id: 'map',
  title: 'Карта',
  requires: ['hsr'],
  screens: [{
    id: 'map',
    title: 'Карта',
    icon: 'map',
    category: 'exploration',
    component: MapPage,
  }],
})
