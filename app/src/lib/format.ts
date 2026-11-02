// Number formatting shared by all screens.

const compact = new Intl.NumberFormat('ru-RU', { notation: 'compact', maximumFractionDigits: 2 })
const full = new Intl.NumberFormat('ru-RU')

/** Missing values (account not linked yet) show as 0. */
export function formatNumber(value: number | null, opts?: { compact?: boolean }): string {
  value ??= 0
  return opts?.compact && value >= 100_000 ? compact.format(value) : full.format(value)
}
