// Relic values are sums of decimal constants (base + step × n), so the exact
// value has few decimals, but its binary approximation may sit just below it:
// 0.162 * 1000 is 161.99999999999997 and a plain floor showed 16,1% for 16,2%.
// The nudge is far above the float error (1e-13) and far below one step of a
// stat (1e-6 of a percent at the smallest).
const NUDGE = 1e-7
const cut = (value: number, scale: number) => Math.floor(value * scale + NUDGE)

/** Stat value as the game shows it: "6,9%", "135", "2,6" for speed. */
export function formatStat(stat: { property: string; value: number; percent: boolean; name?: string; icon?: string; rolls?: number }): string {
  if (stat.percent) return `${(cut(stat.value, 1000) / 10).toLocaleString('ru-RU')}%`
  if (stat.property.startsWith('Speed')) return (cut(stat.value, 10) / 10).toLocaleString('ru-RU')
  return String(cut(stat.value, 1))
}
