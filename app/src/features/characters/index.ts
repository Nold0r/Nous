import { defineFeature } from '../types'
import { CharactersPage } from './CharactersPage'

export default defineFeature({
  id: 'characters',
  title: 'Персонажи',
  icon: 'characters',
  category: 'inventory',
  order: 0,
  component: CharactersPage,
})
