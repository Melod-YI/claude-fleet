export type TerminalType = 'wezterm' | 'cmd' | 'powershell' | 'powershell7'

/// 终端 fallback 默认值：新用户无值、老值不支持（如已移除的 windows-terminal）、
/// createDefaultLaunchSettings 缺省参数，统一引用此常量，避免字面量散落。
/// powershell：Windows 内置、可被系统默认终端路由到 WT。
export const FALLBACK_TERMINAL_TYPE: TerminalType = 'powershell'

export interface CommandWrapperSettings {
  enabled: boolean
  executable: string
  argsBeforeAgent: string[]
}

export interface LaunchSettings {
  terminalId: string
  claudeExecutable: string
  claudeArgs: string[]
  wrapper?: CommandWrapperSettings
  maximizeWindow?: boolean
}

// 音频文件信息
export interface SoundInfo {
  name: string        // 显示名称
  filename: string    // 文件名（"builtin:default" 表示内置默认）
  isBuiltin?: boolean // 是否为内置音频
}

export interface FavoritePath {
  path: string           // 标准化的路径
  useCount: number       // 使用次数
  lastUsedAt: number     // 最近使用时间戳（毫秒）
  pinned: boolean        // 是否置顶
  pinnedAt: number | null // 置顶时间戳，未置顶时为 null
}

export interface FavoritePaths {
  paths: FavoritePath[]
}

export interface AppSettings {
  favoritePaths: FavoritePaths
  defaultTimeRange: '3d' | '7d' | '30d' | 'all'
  notificationSound: boolean
  notificationDesktop: boolean
  notificationSoundFile: string  // 选中的音频文件名（空表示使用默认）
  theme: 'light' | 'dark' | 'system'
  terminalType: TerminalType
  launchSettings: LaunchSettings
}

export function createDefaultLaunchSettings(terminalId: string = FALLBACK_TERMINAL_TYPE): LaunchSettings {
  return {
    terminalId,
    claudeExecutable: 'claude',
    claudeArgs: ['--permission-mode', 'bypassPermissions'],
    wrapper: {
      enabled: false,
      executable: 'ccglass',
      argsBeforeAgent: [],
    },
    maximizeWindow: false,
  }
}

export function parseLaunchSettings(value: string | undefined, fallbackTerminalId: string): LaunchSettings {
  if (!value) {
    return createDefaultLaunchSettings(fallbackTerminalId)
  }

  try {
    const parsed = JSON.parse(value) as Partial<LaunchSettings>
    return {
      terminalId: typeof parsed.terminalId === 'string' && parsed.terminalId.trim()
        ? parsed.terminalId
        : fallbackTerminalId,
      claudeExecutable: typeof parsed.claudeExecutable === 'string' && parsed.claudeExecutable.trim()
        ? parsed.claudeExecutable
        : 'claude',
      claudeArgs: Array.isArray(parsed.claudeArgs)
        ? parsed.claudeArgs.filter((arg): arg is string => typeof arg === 'string')
        : ['--permission-mode', 'bypassPermissions'],
      wrapper: parsed.wrapper && typeof parsed.wrapper === 'object'
        ? {
            enabled: parsed.wrapper.enabled === true,
            executable: typeof parsed.wrapper.executable === 'string' && parsed.wrapper.executable.trim()
              ? parsed.wrapper.executable
              : 'ccglass',
            argsBeforeAgent: Array.isArray(parsed.wrapper.argsBeforeAgent)
              ? parsed.wrapper.argsBeforeAgent.filter((arg): arg is string => typeof arg === 'string')
              : [],
          }
        : {
            enabled: false,
            executable: 'ccglass',
            argsBeforeAgent: [],
          },
      maximizeWindow: parsed.maximizeWindow === true,
    }
  } catch {
    return createDefaultLaunchSettings(fallbackTerminalId)
  }
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