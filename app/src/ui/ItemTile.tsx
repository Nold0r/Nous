/**
 * Small square tile of a stackable item (material, consumable): picture on
 * the rarity background, count, name under it, rarity line along the top
 * — the same rarity look as ItemCard and PortraitCard.
 */
export function ItemTile({ image, rarity, count, name }: { image: string; rarity: number; count: string; name: string }) {
  const r = Math.min(5, Math.max(1, rarity))
  return (
    <div className={`ui-item-tile ui-rarity-tile ui-edge ui-r${r}`} title={name}>
      <img src={image} alt="" draggable={false} loading="lazy" />
      <span className="ui-item-tile__count">{count}</span>
      <span className="ui-item-tile__name">{name}</span>
    </div>
  )
}
