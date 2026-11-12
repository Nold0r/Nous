import { lazy } from 'react'

// The screen (page.tsx) and what it imports load when the tab is first opened,
// not with the application.
export default lazy(() => import('./page'))

export { Palette as icon } from 'lucide-react'
