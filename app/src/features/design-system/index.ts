import { Palette } from 'lucide-react'
import { defineModule } from '../types'
import { DesignSystemPage } from './DesignSystemPage'

// Live catalogue of the design system: every token and component in one place.
// Dev builds only.
export default defineModule({
  id: 'design-system',
  title: 'Дизайн-система',
  requires: [],
  devOnly: true,
  screens: [{
    id: 'design-system',
    title: 'Дизайн-система',
    icon: Palette,
    category: 'system',
    order: -1,
    badge: 'dev',
    devOnly: true,
    component: DesignSystemPage,
  }],
})
