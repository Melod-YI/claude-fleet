import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { AppSettings } from '@/types'

interface SettingsState extends AppSettings {
  addFavoritePath: (path: string) => void
  removeFavoritePath: (path: string) => void
  setDefaultTimeRange: (range: '3d' | '7d' | '30d' | 'all') => void
  setNotificationSound: (enabled: boolean) => void
  setNotificationDesktop: (enabled: boolean) => void
  setTheme: (theme: 'light' | 'dark' | 'system') => void
}

const DEFAULT_SETTINGS: AppSettings = {
  favoritePaths: { paths: [] },
  defaultTimeRange: '30d',
  notificationSound: true,
  notificationDesktop: true,
  theme: 'system',
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...DEFAULT_SETTINGS,

      addFavoritePath: (path: string) => {
        set((state) => ({
          favoritePaths: {
            paths: [...state.favoritePaths.paths, path],
          },
        }))
      },

      removeFavoritePath: (path: string) => {
        set((state) => ({
          favoritePaths: {
            paths: state.favoritePaths.paths.filter((p) => p !== path),
          },
        }))
      },

      setDefaultTimeRange: (range) => set({ defaultTimeRange: range }),
      setNotificationSound: (enabled) => set({ notificationSound: enabled }),
      setNotificationDesktop: (enabled) => set({ notificationDesktop: enabled }),
      setTheme: (theme) => set({ theme }),
    }),
    {
      name: 'claude-fleet-settings',
    }
  )
)