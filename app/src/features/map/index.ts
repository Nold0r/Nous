import { defineFeature } from '../types'
import { MapPage } from './MapPage'

// Map of the current location (placeholder for now).
export default defineFeature({
  id: 'map',
  title: 'Карта',
  icon: 'map',
  category: 'exploration',
  component: MapPage,
})
