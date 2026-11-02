/** Rarity stars coloured like in the game (5★ gold, 4★ purple, 3★ blue). */
export function Rarity({ stars }: { stars: 1 | 2 | 3 | 4 | 5 }) {
  return (
    <span className={`ui-rarity ui-rarity--${stars}`} aria-label={`${stars}★`}>
      {'★'.repeat(stars)}
    </span>
  )
}
