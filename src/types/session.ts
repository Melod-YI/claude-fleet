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
}

// Session message (aligned with cc-switch)
export interface SessionMessage {
  role: string;                // "user", "assistant", "tool"
  content: string;             // Extracted text content
  ts?: number;                 // i64 milliseconds timestamp
}

// 旧类型定义 - 保留用于 running sessions
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