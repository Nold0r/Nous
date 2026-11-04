import type { Battle, ColumnKey, Combatant, OverlaySettings } from './types'

export const viewLabels = { legend: 'Таблица персонажей', characters: 'Урон персонажей', breakdown: 'Типы урона', enemies: 'Противники', metrics: 'Показатели боя', graph: 'График урона', logs: 'Журнал соединения' }
export const columnLabels: Record<ColumnKey, string> = { damage: 'Урон', dpav: 'Урон / AV', share: 'Доля', effective: 'Эффективный', overkill: 'Избыточный', turns: 'Ходы с уроном' }
export const categoryNames: Record<string, string> = { Basic: 'Базовая атака', Skill: 'Навык', Ultimate: 'Сверхспособность', QTE: 'QTE', DoT: 'Периодический', Additional: 'Дополнительный', Technique: 'Техника', 'Follow-Up': 'Бонус-атака', Break: 'Пробитие', Servant: 'Дух памяти', True: 'Истинный', Elation: 'Радость', Assist: 'Поддержка' }
export const propertyNames: Record<string, string> = { CurrentHP: 'Текущие HP', MaxHP: 'Макс. HP', Attack: 'Сила атаки', Defence: 'Защита', Speed: 'Скорость', ActionDelay: 'Задержка действия', CriticalChance: 'Крит. шанс', CriticalDamage: 'Крит. урон', CurrentSP: 'Энергия', MaxSP: 'Макс. энергия' }
export const seriesColors = ['var(--gold)', 'var(--info)', 'var(--success)', 'var(--rarity-4)', 'var(--danger)', 'var(--warning)', 'var(--crimson)']

const number = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 1 })
const compact = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 1, notation: 'compact' })
export const finite = (value: number | undefined) => typeof value === 'number' && Number.isFinite(value) ? value : 0
export const formatNumber = (value: number | undefined) => number.format(finite(value))
export const formatCompact = (value: number | undefined) => compact.format(finite(value))
export const dpav = (damage: number, av: number) => av > 0 ? finite(damage / av) : 0
/** Matches upstream max(current AV, current AV + ActionDelay), without inventing missing stats. */
export function projectedAv(currentAv: number, actionDelay: number | undefined): number | null {
  if (!Number.isFinite(currentAv) || typeof actionDelay !== 'number' || !Number.isFinite(actionDelay)) return null
  const projected = Math.max(currentAv, currentAv + actionDelay)
  return Number.isFinite(projected) ? projected : null
}
export const share = (part: number, total: number) => total > 0 ? finite(part / total * 100) : 0
export const formatShare = (part: number, total: number) => `${formatNumber(share(part, total))}%`
export const effective = (avatar: Combatant) => Math.max(0, finite(avatar.damage) - finite(avatar.overkill))
export const categoryLabel = (value: string) => categoryNames[value] ?? value
export const propertyLabel = (value: string) => propertyNames[value] ?? value

export function damageCategories(battle: Battle, characterId: number | null) {
  const values: Record<string, number> = {}
  for (const avatar of battle.avatars) {
    if (characterId !== null && avatar.id !== characterId) continue
    for (const [key, value] of Object.entries(avatar.byType)) values[key] = (values[key] ?? 0) + Math.max(0, finite(value))
  }
  return Object.entries(values).filter(([, damage]) => damage > 0).sort((a, b) => b[1] - a[1])
}

export function characterValue(avatar: Combatant, column: ColumnKey, battle: Battle): string {
  switch (column) {
    case 'damage': return formatNumber(avatar.damage)
    case 'dpav': return formatNumber(dpav(avatar.damage, battle.actionValue))
    case 'share': return formatShare(avatar.damage, battle.totalDamage)
    case 'effective': return formatNumber(effective(avatar))
    case 'overkill': return formatNumber(avatar.overkill)
    case 'turns': {
      const index = battle.avatars.indexOf(avatar)
      return String(battle.turnHistory.filter((turn) => finite(turn.avatarsTurnDamage[index]) > 0).length)
    }
  }
}

export function sortedAvatars(battle: Battle, metric: OverlaySettings['metric']) {
  return [...battle.avatars].sort((a, b) => metric === 'dpav'
    ? dpav(b.damage, battle.actionValue) - dpav(a.damage, battle.actionValue)
    : b.damage - a.damage)
}

/** Runtime styles remain scoped to the overlay/preview and inherit Nous typography. */
export function themeVariables(settings: OverlaySettings): Record<string, string | number> {
  const vars: Record<string, string | number> = {
    '--vt-opacity': settings.opacity,
    '--vt-body-size': `${settings.fontSizes.body ?? 14}px`,
    '--vt-heading-size': `${settings.fontSizes.heading ?? 20}px`,
    '--vt-small-size': `${settings.fontSizes.small ?? 12}px`,
    '--vt-legend-size': `${settings.legendSize === 'small' ? settings.fontSizes.small ?? 12 : settings.legendSize === 'large' ? settings.fontSizes.heading ?? 20 : settings.fontSizes.legend ?? 14}px`,
  }
  return vars
}
