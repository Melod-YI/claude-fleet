export interface FavoritePaths {
  paths: string[]
}

export interface AppSettings {
  favoritePaths: FavoritePaths
  defaultTimeRange: '3d' | '7d' | '30d' | 'all'
  notificationSound: boolean
  notificationDesktop: boolean
  theme: 'light' | 'dark' | 'system'
}