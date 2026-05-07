// Session 运行状态（对应 Claude JSON 文件中的三种状态）
export type SessionStatus = 'busy' | 'idle' | 'waiting'

export interface ClaudeSession {
  id: string
  name: string
  workingDirectory: string
  status: SessionStatus
  createdAt: string  // ISO datetime
  lastActivityAt: string  // ISO datetime
  conversationCount: number
  isFavorite: boolean
  terminalWindowId?: string  // Windows Terminal 窗口标识
  processId?: number  // Claude 进程 ID
}

export interface SessionFilter {
  searchQuery?: string
  showFavoritesOnly: boolean
  timeRange?: '3d' | '7d' | '30d' | 'all'
  status?: SessionStatus
}

export interface SessionCreateOptions {
  workingDirectory: string
  name?: string
  addToFavorites: boolean
}