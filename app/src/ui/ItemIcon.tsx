import { itemIcons } from '../lib/icons'

/** Full-colour game item icon, e.g. `<ItemIcon name="stellar-jade" />`. */
export function ItemIcon({ name, size = 24, title }: { name: string; size?: number; title?: string }) {
  const src = itemIcons[name]
  if (!src) console.warn(`[icons] unknown item icon "${name}"`)
  return (
    <img className="item-icon" src={src} width={size} height={size} alt={title ?? ''} title={title} draggable={false} />
  )
}
