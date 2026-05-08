import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { AppSettings, FavoritePath, TerminalType } from '@/types'
import { FAVORITE_PATH_CONFIG } from '@/types'

interface SettingsState extends AppSettings {
  // 常用路径操作
  recordPathUsage: (path: string) => void
  removeFavoritePath: (path: string) => void
  // 设置操作
  setDefaultTimeRange: (range: '3d' | '7d' | '30d' | 'all') => void
  setNotificationSound: (enabled: boolean) => void
  setNotificationDesktop: (enabled: boolean) => void
  setTheme: (theme: 'light' | 'dark' | 'system') => void
  setTerminalType: (type: TerminalType) => void
  // 获取排序后的常用路径
  getSortedFavoritePaths: () => FavoritePath[]
}

const DEFAULT_SETTINGS: AppSettings = {
  favoritePaths: { paths: [] },
  defaultTimeRange: '30d',
  notificationSound: true,
  notificationDesktop: true,
  theme: 'system',
  terminalType: 'wezterm',
}

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

/**
 * 计算路径排序分数
 * 综合考虑：最近使用时间 + 使用频率
 */
function calculatePathScore(path: FavoritePath): number {
  const now = Date.now()
  const daysSinceLastUse = (now - path.lastUsedAt) / (1000 * 60 * 60 * 24)

  // 时间衰减：超过 decayDays 后权重衰减
  const recencyDecayDays = FAVORITE_PATH_CONFIG.recencyDecayDays
  const recencyFactor = Math.exp(-daysSinceLastUse / recencyDecayDays)

  // 使用次数归一化（log 函数避免次数差异过大）
  const frequencyFactor = Math.log10(path.useCount + 1) / Math.log10(100)

  // 综合分数
  const score =
    recencyFactor * FAVORITE_PATH_CONFIG.recencyWeight +
    frequencyFactor * FAVORITE_PATH_CONFIG.frequencyWeight

  return score
}

/**
 * 排序常用路径并限制数量
 */
function sortAndLimitPaths(paths: FavoritePath[]): FavoritePath[] {
  // 按分数降序排序
  const sorted = [...paths]
    .sort((a, b) => calculatePathScore(b) - calculatePathScore(a))
    .slice(0, FAVORITE_PATH_CONFIG.maxDisplay)

  return sorted
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set, get) => ({
      ...DEFAULT_SETTINGS,

      /**
       * 记录路径使用（新建 session 时调用）
       * - 标准化路径
       * - 更新使用次数和时间
       * - 去重
       */
      recordPathUsage: (path: string) => {
        const normalized = normalizePath(path)
        const now = Date.now()
        const existingPaths = get().favoritePaths.paths

        // 查找是否已存在
        const existingIndex = existingPaths.findIndex(
          (p) => normalizePath(p.path) === normalized
        )

        if (existingIndex >= 0) {
          // 已存在：更新次数和时间
          const updatedPaths = [...existingPaths]
          updatedPaths[existingIndex] = {
            ...updatedPaths[existingIndex],
            path: normalized, // 使用标准化路径
            useCount: updatedPaths[existingIndex].useCount + 1,
            lastUsedAt: now,
          }
          set({ favoritePaths: { paths: updatedPaths } })
        } else {
          // 新路径：添加
          set({
            favoritePaths: {
              paths: [
                ...existingPaths,
                { path: normalized, useCount: 1, lastUsedAt: now },
              ],
            },
          })
        }
      },

      /**
       * 移除常用路径
       */
      removeFavoritePath: (path: string) => {
        const normalized = normalizePath(path)
        set((state) => ({
          favoritePaths: {
            paths: state.favoritePaths.paths.filter(
              (p) => normalizePath(p.path) !== normalized
            ),
          },
        }))
      },

      setDefaultTimeRange: (range) => set({ defaultTimeRange: range }),
      setNotificationSound: (enabled) => set({ notificationSound: enabled }),
      setNotificationDesktop: (enabled) => set({ notificationDesktop: enabled }),
      setTheme: (theme) => set({ theme }),
      setTerminalType: (type) => set({ terminalType: type }),

      /**
       * 获取排序后的常用路径（用于显示）
       */
      getSortedFavoritePaths: () => {
        return sortAndLimitPaths(get().favoritePaths.paths)
      },
    }),
    {
      name: 'claude-fleet-settings',
    }
  )
)