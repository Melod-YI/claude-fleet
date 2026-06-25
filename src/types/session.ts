// 新类型定义 - 与 cc-switch 对齐

// Session metadata (aligned with cc-switch)
export interface SessionMeta {
  providerId: string;          // "claude" for this app
  sessionId: string;           // Session UUID
  title?: string;              // First user message or custom title
  summary?: string;            // Last message content (truncated)
  projectDir?: string;         // Working directory
  createdAt?: number;          // i64 milliseconds
  lastActiveAt?: number;       // i64 milliseconds
  sourcePath?: string;         // Full path to JSONL file
  resumeCommand?: string;      // e.g., "claude --resume <sessionId>"
  // Client-side only
  isFavorite?: boolean;
  customName?: string;         // Claude Fleet 自定义名称
}

// Session message (aligned with cc-switch)
export interface SessionMessage {
  role: string;                // "user", "assistant", "tool"
  content: string;             // Extracted text content
  ts?: number;                 // i64 milliseconds timestamp
}

// Session 运行状态（对应 Claude JSON 文件中的三种状态）
export type SessionStatus = 'busy' | 'idle' | 'waiting'

// 工作目录的 git 概要信息（snake_case，与后端 RunningSession 一致）
export interface GitInfo {
  branch: string
  is_detached: boolean
  is_worktree: boolean
  worktree_name?: string
  ahead: number
  behind: number
  dirty: boolean
  last_commit_sha: string
  last_commit_message: string
}

// Running session (from Tauri backend)
export interface RunningSession {
  session_id: string
  pid: number
  status: SessionStatus
  cwd: string
  name: string
  updated_at: number
  away_summary?: string
  away_summary_at?: number
  last_user_input?: string
  custom_name?: string       // Claude Fleet 自定义名称
  git_info?: GitInfo
}

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