export type TerminalType = 'wezterm' | 'cmd' | 'powershell'

export interface FavoritePath {
  path: string           // 标准化的路径
  useCount: number       // 使用次数
  lastUsedAt: number     // 最近使用时间戳（毫秒）
}

export interface FavoritePaths {
  paths: FavoritePath[]
}

export interface AppSettings {
  favoritePaths: FavoritePaths
  defaultTimeRange: '3d' | '7d' | '30d' | 'all'
  notificationSound: boolean
  notificationDesktop: boolean
  theme: 'light' | 'dark' | 'system'
  terminalType: TerminalType
}

// 常用路径排序权重配置
export const FAVORITE_PATH_CONFIG = {
  // 最大显示数量
  maxDisplay: 10,
  // 排序权重：最近使用时间权重 vs 使用次数权重
  recencyWeight: 0.6,
  frequencyWeight: 0.4,
  // 时间衰减因子（天数），超过此天数后最近使用权重衰减
  recencyDecayDays: 30,
}