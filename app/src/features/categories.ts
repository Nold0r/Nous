import type { CategoryDef } from './types'

// Sidebar sections, in display order. To add one, append an entry here, then
// point features at its id (the id becomes part of the CategoryId type, so a
// typo in a feature's `category` is a compile error).
// Docs: docs/Архитектура/Категории меню.md
export const categories = [
  { id: 'main', order: 0 },
  { id: 'exploration', title: 'Исследование', order: 10 },
  { id: 'inventory', title: 'Инвентарь', order: 20 },
  { id: 'system', order: 1000, pinBottom: true },
] as const satisfies readonly CategoryDef[]

export type CategoryId = (typeof categories)[number]['id']
