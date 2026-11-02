import { defineModule } from '../types'
import { CharactersPage } from './CharactersPage'

export default defineModule({
  id: 'characters',
  title: 'Персонажи',
  requires: ['hsr'],
  screens: [{
    id: 'characters',
    title: 'Персонажи',
    icon: 'characters',
    category: 'inventory',
    order: 0,
    component: CharactersPage,
  }],
})
