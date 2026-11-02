export interface Combatant {
  id: number
  name: string
  damage: number
  overkill: number
  byType: Record<string, number>
  properties: Record<string, number>
  dead: boolean
}

export interface TurnInfo {
  actionValue: number
  cycle: number
  wave: number
  avatarsTurnDamage: number[]
  totalDamage: number
}

export interface Battle {
  id: number
  startedAt: number
  endedAt: number | null
  interrupted: boolean
  partial: boolean
  stageId: number
  avatars: Combatant[]
  enemies: { uid: number; id: number; name: string; properties: Record<string, number>; dead: boolean }[]
  turnHistory: TurnInfo[]
  avHistory: TurnInfo[]
  skills: { avatarId: number; name: string; skillType: string; skillConfigId: number; damage: number; details: { damage: number; overkillDamage: number; type: string }[]; turnIndex: number }[]
  entityTurns: { owner: { uid: number; team: 'Player' | 'Enemy' }; actionValue: number; wave: number; cycle: number }[]
  totalDamage: number
  actionValue: number
  wave: number
  cycle: number
  maxWaves: number
  maxCycles: number
}

export interface BattleSummary {
  id: number
  stageId: number
  totalDamage: number
  actionValue: number
  endedAt: number | null
  interrupted: boolean
  partial: boolean
}

export interface Snapshot {
  revision: number
  connected: boolean
  active: boolean
  battle: Battle | null
  history: BattleSummary[]
  diagnostics: string[]
}

export type ViewKey = 'legend' | 'characters' | 'breakdown' | 'enemies' | 'metrics' | 'graph' | 'logs'
export type ColumnKey = 'damage' | 'dpav' | 'share' | 'effective' | 'overkill' | 'turns'
export type ThemeName = 'nous' | 'light' | 'indigo-jade' | 'grass-bronze' | 'warm' | 'cool' | 'seventies' | 'office-gray' | 'vermillion'

export interface OverlaySettings {
  enabled: boolean
  clickThrough: boolean
  scale: number
  opacity: number
  width: number
  height: number
  anchor: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left'
  offsetX: number
  offsetY: number
  theme: ThemeName
  fontSizes: Record<string, number>
  legendSize: 'small' | 'body' | 'large'
  pieOpacity: number
  views: Record<ViewKey, boolean>
  columns: ColumnKey[]
  metric: 'damage' | 'dpav'
  breakdownScope: 'team' | 'character'
  breakdownChart: 'pie' | 'bar'
  characterId: number | null
  showLegend: boolean
  showBreakdownTable: boolean
  graphUnit: 'turn' | 'action-value'
  streamerMode: boolean
  streamerMessage: string
  autoExport: boolean
  dateFolders: boolean
  exportDirectory: string | null
}

export interface OverlayState {
  snapshot: Snapshot
  settings: OverlaySettings
  presentation: { targetFound: boolean; targetVisible: boolean; exclusiveFullscreen: boolean; visible: boolean; message: string | null }
  replay: boolean
  hidden: boolean
}

export interface PluginOptions {
  locale: string
  streamerMode: boolean
  streamerMessage: string
  widgetOpacity: number
  pieOpacity: number
  autoShowHide: boolean
  themeMode: 'Dark' | 'Light'
  legendTextStyle: string
  fontSizes: Record<string, number>
  suppressNative: boolean
}

export interface PluginConfig {
  directory: string
  options: PluginOptions
  requiresRestart: true
  warning: string
}

export const defaultSettings: OverlaySettings = {
  enabled: true, clickThrough: true, scale: 1, opacity: .86, width: 520, height: 620,
  anchor: 'top-right', offsetX: 24, offsetY: 24, theme: 'nous',
  fontSizes: { body: 14, heading: 20, small: 12, legend: 14 }, legendSize: 'body', pieOpacity: .8,
  views: { legend: true, characters: true, breakdown: false, enemies: false, metrics: true, graph: false, logs: false },
  columns: ['damage', 'dpav', 'share'], metric: 'damage', breakdownScope: 'team', breakdownChart: 'pie',
  characterId: null, showLegend: true, showBreakdownTable: true, graphUnit: 'turn', streamerMode: false,
  streamerMessage: 'Nous', autoExport: false, dateFolders: true, exportDirectory: null,
}
