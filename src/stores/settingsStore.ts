import { create } from 'zustand'
import {
  setSetting,
  getAllSettings,
  recordPathUsage,
  removeFavoritePath,
  getSortedFavoritePaths,
  FavoritePath,
} from '@/services/dbService'
import type { AppSettings, TerminalType } from '@/types'

interface SettingsState extends AppSettings {
  initialized: boolean

  // Actions
  initialize: () => Promise<void>
  recordPathUsage: (path: string) => Promise<void>
  removeFavoritePath: (path: string) => Promise<void>
  setDefaultTimeRange: (range: '3d' | '7d' | '30d' | 'all') => Promise<void>
  setNotificationSound: (enabled: boolean) => Promise<void>
  setNotificationDesktop: (enabled: boolean) => Promise<void>
  setNotificationSoundFile: (filename: string) => Promise<void>
  setTheme: (theme: 'light' | 'dark' | 'system') => Promise<void>
  setTerminalType: (type: TerminalType) => Promise<void>
  getSortedFavoritePaths: () => FavoritePath[]
}

const DEFAULT_SETTINGS: AppSettings = {
  favoritePaths: { paths: [] },
  defaultTimeRange: '30d',
  notificationSound: true,
  notificationDesktop: true,
  notificationSoundFile: '',
  theme: 'system',
  terminalType: 'wezterm',
}

export const useSettingsStore = create<SettingsState>()((set, get) => ({
  ...DEFAULT_SETTINGS,
  initialized: false,

  initialize: async () => {
    try {
      const settings = await getAllSettings()

      const parsed: Partial<AppSettings> = {}

      if (settings['defaultTimeRange']) {
        parsed.defaultTimeRange = settings['defaultTimeRange'] as '3d' | '7d' | '30d' | 'all'
      }
      if (settings['notificationSound']) {
        parsed.notificationSound = settings['notificationSound'] === 'true'
      }
      if (settings['notificationDesktop']) {
        parsed.notificationDesktop = settings['notificationDesktop'] === 'true'
      }
      if (settings['notificationSoundFile']) {
        parsed.notificationSoundFile = settings['notificationSoundFile']
      }
      if (settings['theme']) {
        parsed.theme = settings['theme'] as 'light' | 'dark' | 'system'
      }
      if (settings['terminalType']) {
        parsed.terminalType = settings['terminalType'] as TerminalType
      }

      const paths = await getSortedFavoritePaths()
      parsed.favoritePaths = { paths }

      set({ ...DEFAULT_SETTINGS, ...parsed, initialized: true })
    } catch (e) {
      console.error('初始化设置失败:', e)
      set({ ...DEFAULT_SETTINGS, initialized: true })
    }
  },

  recordPathUsage: async (path: string) => {
    const normalized = normalizePath(path)
    await recordPathUsage(normalized)
    const paths = await getSortedFavoritePaths()
    set({ favoritePaths: { paths } })
  },

  removeFavoritePath: async (path: string) => {
    const normalized = normalizePath(path)
    await removeFavoritePath(normalized)
    const paths = await getSortedFavoritePaths()
    set({ favoritePaths: { paths } })
  },

  setDefaultTimeRange: async (range) => {
    await setSetting('defaultTimeRange', range)
    set({ defaultTimeRange: range })
  },

  setNotificationSound: async (enabled) => {
    await setSetting('notificationSound', enabled.toString())
    set({ notificationSound: enabled })
  },

  setNotificationDesktop: async (enabled) => {
    await setSetting('notificationDesktop', enabled.toString())
    set({ notificationDesktop: enabled })
  },

  setNotificationSoundFile: async (filename) => {
    await setSetting('notificationSoundFile', filename)
    set({ notificationSoundFile: filename })
  },

  setTheme: async (theme) => {
    await setSetting('theme', theme)
    set({ theme })
  },

  setTerminalType: async (type) => {
    await setSetting('terminalType', type)
    set({ terminalType: type })
  },

  getSortedFavoritePaths: () => {
    return get().favoritePaths.paths
  },
}))

/**
 * 标准化路径（去除末尾斜杠、统一大小写等）
 */
function normalizePath(path: string): string {
  let normalized = path.trim()
  // Windows 路径：去除末尾的 \ 或 /
  if (normalized.length > 3) {
    normalized = normalized.replace(/[\\\/]+$/, '')
  }
  // Windows 路径大小写统一（驱动器字母大写）
  if (normalized.match(/^[a-zA-Z]:/)) {
    normalized = normalized[0].toUpperCase() + normalized.slice(1)
  }
  return normalized
}