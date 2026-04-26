# Phase 2: Session 数据层

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Claude session 数据读取、存储和管理能力

**Architecture:** Tauri 后端读取 Claude 原生 session 数据，前端通过 Tauri commands 获取数据，Zustand 管理状态

**Tech Stack:** Tauri commands (Rust), Zustand, TypeScript

---

## Task 2.1: 调研 Claude Session 存储位置

**Files:**
- Create: `docs/research/claude-session-storage.md`

- [ ] **Step 1: 查找 Claude Code session 存储位置**

Claude Code session 数据存储在用户目录下：
- Windows: `C:\Users\<username>\.claude\projects\<project-hash>\sessions\`
- macOS: `~/.claude/projects/<project-hash>/sessions/`
- Linux: `~/.claude/projects/<project-hash>/sessions/`

Session 文件格式：
- 每个 session 是一个 JSONL 文件
- 文件名格式: `<session-id>.jsonl`
- 每行是一个 JSON 对象，包含消息内容

- [ ] **Step 2: 验证存储位置**

```bash
# Windows
ls C:\Users\Melodyi\.claude\projects\
```

Expected: 显示项目目录列表

- [ ] **Step 3: 查看 session 文件结构**

```bash
# 选择一个项目目录查看
ls C:\Users\Melodyi\.claude\projects\<project-hash>\sessions\
```

Expected: 显示 session 文件列表

- [ ] **Step 4: 创建调研文档**

创建 `docs/research/claude-session-storage.md`：

```markdown
# Claude Code Session 存储结构

## 存储位置

- Windows: `C:\Users\<username>\.claude\projects\<project-hash>\sessions\`
- macOS/Linux: `~/.claude/projects/<project-hash>/sessions/`

## 项目哈希计算

项目哈希是工作目录路径的某种编码（需进一步调研）。

## Session 文件格式

每个 session 是 JSONL 文件，每行包含：
- 消息内容
- 时间戳
- 角色信息

## 相关文件

- `projects.json`: 项目列表和元数据
- `session_metadata.json`: Session 元数据（如果存在）

## 待确认

1. 项目哈希的计算方式
2. 是否有 session 元数据索引文件
3. 运行中 session 的标识方式（进程 ID 等）
```

- [ ] **Step 5: Commit**

```bash
git add docs/research/
git commit -m "docs: 调研 Claude Code session 存储结构"
```

---

## Task 2.2: 创建 Rust Claude 数据读取模块

**Files:**
- Create: `src-tauri/src/utils/claude_data.rs`
- Create: `src-tauri/src/utils/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 utils 目录**

```bash
mkdir -p src-tauri/src/utils
```

- [ ] **Step 2: 创建 utils 模块入口**

创建 `src-tauri/src/utils/mod.rs`：

```rust
pub mod claude_data;
```

- [ ] **Step 3: 定义 Session 数据结构**

创建 `src-tauri/src/utils/claude_data.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSession {
    pub id: String,
    pub name: String,
    pub working_directory: String,
    pub status: String,
    pub created_at: String,
    pub last_activity_at: String,
    pub conversation_count: u32,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_window_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub session_id: String,
    pub messages: Vec<ConversationMessage>,
    pub total_messages: u32,
}

/// 获取 Claude 数据根目录
pub fn get_claude_data_dir() -> PathBuf {
    // Windows: C:\Users\<username>\.claude
    // macOS/Linux: ~/.claude
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude")
}

/// 获取项目目录列表
pub fn get_projects_dir() -> PathBuf {
    get_claude_data_dir().join("projects")
}

/// 解析单个 session 文件
pub fn parse_session_file(file_path: &PathBuf) -> Result<(ClaudeSession, Conversation), String> {
    let file = fs::File::open(file_path)
        .map_err(|e| format!("无法打开文件: {}", e))?;

    let reader = BufReader::new(file);
    let mut messages: Vec<ConversationMessage> = Vec::new();
    let mut created_at = String::new();
    let mut last_activity_at = String::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("读取行失败: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }

        // 尝试解析 JSON
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
            let role = json_value.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let content = json_value.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = json_value.get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if index == 0 && timestamp.is_empty() {
                created_at = timestamp.clone();
            }
            last_activity_at = timestamp.clone();

            messages.push(ConversationMessage {
                id: format!("msg-{}", index),
                role,
                content,
                timestamp,
            });
        }
    }

    // 从文件名提取 session ID
    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // 工作目录需要从其他地方获取（后续实现）
    let working_directory = String::new();

    let session = ClaudeSession {
        id: session_id.clone(),
        name: session_id.clone(), // 默认使用 ID，后续可自定义
        working_directory,
        status: "idle".to_string(), // 默认状态，后续需要检测运行状态
        created_at,
        last_activity_at,
        conversation_count: messages.len() as u32,
        is_favorite: false,
        terminal_window_id: None,
        process_id: None,
    };

    let conversation = Conversation {
        session_id,
        messages,
        total_messages: messages.len() as u32,
    };

    Ok((session, conversation))
}

/// 获取所有 session 列表
pub fn get_all_sessions() -> Result<Vec<ClaudeSession>, String> {
    let projects_dir = get_projects_dir();

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions: Vec<ClaudeSession> = Vec::new();

    // 遍历所有项目目录
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| format!("读取项目目录失败: {}", e))?
    {
        let project_dir = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        let sessions_dir = project_dir.join("sessions");

        if !sessions_dir.exists() {
            continue;
        }

        // 遍历所有 session 文件
        for session_entry in fs::read_dir(&sessions_dir)
            .map_err(|e| format!("读取 sessions 目录失败: {}", e))?
        {
            let session_file = session_entry
                .map_err(|e| format!("读取 session 条目失败: {}", e))?
                .path();

            if session_file.extension().and_then(|s| s.to_str()) != Some("jsonl") {
                continue;
            }

            if let Ok((session, _)) = parse_session_file(&session_file) {
                sessions.push(session);
            }
        }
    }

    Ok(sessions)
}

/// 获取指定 session 的对话内容
pub fn get_session_conversation(session_id: &str) -> Result<Conversation, String> {
    let projects_dir = get_projects_dir();

    // 需要遍历找到对应 session 文件
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| format!("读取项目目录失败: {}", e))?
    {
        let project_dir = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        let sessions_dir = project_dir.join("sessions");

        if !sessions_dir.exists() {
            continue;
        }

        let session_file = sessions_dir.join(format!("{}.jsonl", session_id));

        if session_file.exists() {
            let (_, conversation) = parse_session_file(&session_file)?;
            return Ok(conversation);
        }
    }

    Err(format!("Session {} 不存在", session_id))
}
```

- [ ] **Step 4: 更新 lib.rs 引入模块**

编辑 `src-tauri/src/lib.rs`：

```rust
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 添加 dirs 依赖**

已在 Phase 1 的 Cargo.toml 中添加。

- [ ] **Step 6: 编译验证**

```bash
cargo build
```

Expected: 编译成功，无错误

- [ ] **Step 7: Commit**

```bash
git add .
git commit -m "feat: 创建 Rust Claude 数据读取模块"
```

---

## Task 2.3: 创建 Tauri Commands

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/session.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建 commands 目录**

```bash
mkdir -p src-tauri/src/commands
```

- [ ] **Step 2: 创建 commands 模块入口**

创建 `src-tauri/src/commands/mod.rs`：

```rust
pub mod session;
```

- [ ] **Step 3: 创建 session commands**

创建 `src-tauri/src/commands/session.rs`：

```rust
use crate::utils::claude_data::{get_all_sessions, get_session_conversation, ClaudeSession, Conversation};

#[tauri::command]
pub fn list_sessions() -> Result<Vec<ClaudeSession>, String> {
    get_all_sessions()
}

#[tauri::command]
pub fn get_conversation(session_id: String) -> Result<Conversation, String> {
    get_session_conversation(&session_id)
}

#[tauri::command]
pub fn refresh_sessions() -> Result<Vec<ClaudeSession>, String> {
    get_all_sessions()
}
```

- [ ] **Step 4: 更新 lib.rs 注册 commands**

编辑 `src-tauri/src/lib.rs`：

```rust
mod utils;
mod commands;

use commands::session::{list_sessions, get_conversation, refresh_sessions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_conversation,
            refresh_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 编译验证**

```bash
cargo build
```

Expected: 编译成功

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 创建 Tauri session commands"
```

---

## Task 2.4: 创建前端 Session 服务层

**Files:**
- Create: `src/services/claudeSession.ts`
- Create: `src/services/index.ts`

- [ ] **Step 1: 创建 services 目录**

```bash
mkdir -p src/services
```

- [ ] **Step 2: 创建 Claude Session 服务**

创建 `src/services/claudeSession.ts`：

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession, Conversation } from '@/types'

export async function listSessions(): Promise<ClaudeSession[]> {
  try {
    const sessions = await invoke<ClaudeSession[]>('list_sessions')
    return sessions
  } catch (error) {
    console.error('获取 session 列表失败:', error)
    throw error
  }
}

export async function getConversation(sessionId: string): Promise<Conversation> {
  try {
    const conversation = await invoke<Conversation>('get_conversation', { sessionId })
    return conversation
  } catch (error) {
    console.error('获取对话内容失败:', error)
    throw error
  }
}

export async function refreshSessions(): Promise<ClaudeSession[]> {
  try {
    const sessions = await invoke<ClaudeSession[]>('refresh_sessions')
    return sessions
  } catch (error) {
    console.error('刷新 session 列表失败:', error)
    throw error
  }
}
```

- [ ] **Step 3: 创建 services 入口**

创建 `src/services/index.ts`：

```typescript
export * from './claudeSession'
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建前端 session 服务层"
```

---

## Task 2.5: 创建 Session Zustand Store

**Files:**
- Create: `src/stores/sessionStore.ts`
- Modify: `src/stores/index.ts`

- [ ] **Step 1: 创建 session store**

创建 `src/stores/sessionStore.ts`：

```typescript
import { create } from 'zustand'
import type { ClaudeSession, Conversation, SessionFilter } from '@/types'
import { listSessions, getConversation, refreshSessions } from '@/services'

interface SessionState {
  sessions: ClaudeSession[]
  selectedSessionId: string | null
  currentConversation: Conversation | null
  filter: SessionFilter
  loading: boolean
  error: string | null

  // Actions
  loadSessions: () => Promise<void>
  selectSession: (sessionId: string) => Promise<void>
  setFilter: (filter: Partial<SessionFilter>) => void
  refresh: () => Promise<void>
  clearError: () => void
}

export const useSessionStore = create<SessionState>((set, get) => ({
  sessions: [],
  selectedSessionId: null,
  currentConversation: null,
  filter: {
    showFavoritesOnly: true,
    timeRange: '30d',
  },
  loading: false,
  error: null,

  loadSessions: async () => {
    set({ loading: true, error: null })
    try {
      const sessions = await listSessions()
      set({ sessions, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  selectSession: async (sessionId: string) => {
    set({ selectedSessionId: sessionId, loading: true })
    try {
      const conversation = await getConversation(sessionId)
      set({ currentConversation: conversation, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  setFilter: (filter: Partial<SessionFilter>) => {
    set((state) => ({
      filter: { ...state.filter, ...filter }
    }))
  },

  refresh: async () => {
    set({ loading: true })
    try {
      const sessions = await refreshSessions()
      set({ sessions, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  clearError: () => set({ error: null }),
}))
```

- [ ] **Step 2: 更新 stores 入口**

编辑 `src/stores/index.ts`：

```typescript
export { useSessionStore } from './sessionStore'
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建 session Zustand store"
```

---

## Task 2.6: 创建 Favorite Store

**Files:**
- Create: `src/stores/favoriteStore.ts`
- Modify: `src/stores/index.ts`

- [ ] **Step 1: 创建 favorite store**

创建 `src/stores/favoriteStore.ts`：

```typescript
import { create } from 'zustand'
import { persist } from 'zustand/middleware'

interface FavoriteState {
  favorites: Set<string>

  // Actions
  addFavorite: (sessionId: string) => void
  removeFavorite: (sessionId: string) => void
  toggleFavorite: (sessionId: string) => void
  isFavorite: (sessionId: string) => boolean
}

export const useFavoriteStore = create<FavoriteState>()(
  persist(
    (set, get) => ({
      favorites: new Set<string>(),

      addFavorite: (sessionId: string) => {
        set((state) => {
          const newFavorites = new Set(state.favorites)
          newFavorites.add(sessionId)
          return { favorites: newFavorites }
        })
      },

      removeFavorite: (sessionId: string) => {
        set((state) => {
          const newFavorites = new Set(state.favorites)
          newFavorites.delete(sessionId)
          return { favorites: newFavorites }
        })
      },

      toggleFavorite: (sessionId: string) => {
        const state = get()
        if (state.favorites.has(sessionId)) {
          state.removeFavorite(sessionId)
        } else {
          state.addFavorite(sessionId)
        }
      },

      isFavorite: (sessionId: string) => {
        return get().favorites.has(sessionId)
      },
    }),
    {
      name: 'claude-fleet-favorites',
      // Set 需要特殊序列化
      storage: {
        getItem: (name) => {
          const str = localStorage.getItem(name)
          if (!str) return null
          const data = JSON.parse(str)
          return {
            ...data,
            state: {
              ...data.state,
              favorites: new Set(data.state.favorites || []),
            },
          }
        },
        setItem: (name, value) => {
          const data = {
            ...value,
            state: {
              ...value.state,
              favorites: Array.from(value.state.favorites),
            },
          }
          localStorage.setItem(name, JSON.stringify(data))
        },
        removeItem: (name) => localStorage.removeItem(name),
      },
    }
  )
)
```

- [ ] **Step 2: 更新 stores 入口**

编辑 `src/stores/index.ts`：

```typescript
export { useSessionStore } from './sessionStore'
export { useFavoriteStore } from './favoriteStore'
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建 favorite Zustand store（带持久化）"
```

---

## Task 2.7: 创建搜索和过滤工具函数

**Files:**
- Create: `src/utils/fuzzySearch.ts`
- Create: `src/utils/timeUtils.ts`
- Create: `src/utils/pathUtils.ts`
- Create: `src/utils/index.ts`

- [ ] **Step 1: 创建 utils 目录**

```bash
mkdir -p src/utils
```

- [ ] **Step 2: 创建模糊搜索函数**

创建 `src/utils/fuzzySearch.ts`：

```typescript
import type { ClaudeSession } from '@/types'

/**
 * 简单的模糊匹配函数
 * 检查 query 是否在 text 中出现（不区分大小写）
 */
export function fuzzyMatch(text: string, query: string): boolean {
  const lowerText = text.toLowerCase()
  const lowerQuery = query.toLowerCase()
  return lowerText.includes(lowerQuery)
}

/**
 * 搜索 session
 * 支持搜索名称、路径、对话内容
 */
export function searchSessions(
  sessions: ClaudeSession[],
  query: string,
  searchableFields: ('name' | 'path' | 'content')[] = ['name', 'path']
): ClaudeSession[] {
  if (!query.trim()) return sessions

  return sessions.filter((session) => {
    if (searchableFields.includes('name') && fuzzyMatch(session.name, query)) {
      return true
    }
    if (searchableFields.includes('path') && fuzzyMatch(session.workingDirectory, query)) {
      return true
    }
    // 对话内容搜索需要额外实现
    return false
  })
}
```

- [ ] **Step 3: 创建时间过滤函数**

创建 `src/utils/timeUtils.ts`：

```typescript
import type { ClaudeSession } from '@/types'

const TIME_RANGES = {
  '3d': 3 * 24 * 60 * 60 * 1000,
  '7d': 7 * 24 * 60 * 60 * 1000,
  '30d': 30 * 24 * 60 * 60 * 1000,
  'all': Infinity,
}

/**
 * 过滤指定时间范围内的 session
 */
export function filterByTimeRange(
  sessions: ClaudeSession[],
  timeRange: '3d' | '7d' | '30d' | 'all'
): ClaudeSession[] {
  if (timeRange === 'all') return sessions

  const now = new Date().getTime()
  const rangeMs = TIME_RANGES[timeRange]

  return sessions.filter((session) => {
    const lastActivity = new Date(session.lastActivityAt).getTime()
    return now - lastActivity <= rangeMs
  })
}

/**
 * 格式化相对时间
 */
export function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()

  const minutes = Math.floor(diffMs / (60 * 1000))
  const hours = Math.floor(diffMs / (60 * 60 * 1000))
  const days = Math.floor(diffMs / (24 * 60 * 60 * 1000))

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  if (hours < 24) return `${hours} 小时前`
  if (days < 7) return `${days} 天前`
  if (days < 30) return `${Math.floor(days / 7)} 周前`

  return date.toLocaleDateString('zh-CN')
}
```

- [ ] **Step 4: 创建路径处理函数**

创建 `src/utils/pathUtils.ts`：

```typescript
/**
 * 构建目录树结构
 */
export interface TreeNode {
  path: string
  name: string
  children: TreeNode[]
  sessionCount: number
  isLeaf: boolean
}

/**
 * 从路径列表构建树结构
 */
export function buildPathTree(paths: string[]): TreeNode {
  const root: TreeNode = {
    path: '',
    name: 'root',
    children: [],
    sessionCount: 0,
    isLeaf: false,
  }

  for (const path of paths) {
    const parts = path.split(/[/\\]/).filter(Boolean)
    let current = root

    for (const part of parts) {
      let child = current.children.find((c) => c.name === part)
      if (!child) {
        child = {
          path: current.path ? `${current.path}/${part}` : part,
          name: part,
          children: [],
          sessionCount: 0,
          isLeaf: false,
        }
        current.children.push(child)
      }
      current = child
    }
  }

  return root
}

/**
 * 获取路径的最后一部分
 */
export function getLastPathSegment(path: string): string {
  const parts = path.split(/[/\\]/).filter(Boolean)
  return parts[parts.length - 1] || path
}
```

- [ ] **Step 5: 创建 utils 入口**

创建 `src/utils/index.ts`：

```typescript
export * from './fuzzySearch'
export * from './timeUtils'
export * from './pathUtils'
```

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 创建搜索、时间过滤、路径处理工具函数"
```

---

## Task 2.8: 创建 useSessions Hook

**Files:**
- Create: `src/hooks/useSessions.ts`
- Create: `src/hooks/index.ts`

- [ ] **Step 1: 创建 hooks 目录**

```bash
mkdir -p src/hooks
```

- [ ] **Step 2: 创建 useSessions hook**

创建 `src/hooks/useSessions.ts`：

```typescript
import { useEffect, useMemo } from 'react'
import { useSessionStore, useFavoriteStore } from '@/stores'
import { searchSessions, filterByTimeRange } from '@/utils'
import type { ClaudeSession } from '@/types'

export function useSessions() {
  const { sessions, filter, loading, error, loadSessions, setFilter } = useSessionStore()
  const { favorites, isFavorite, toggleFavorite } = useFavoriteStore()

  // 初始加载
  useEffect(() => {
    loadSessions()
  }, [loadSessions])

  // 合合收藏状态到 session
  const sessionsWithFavorites = useMemo(() => {
    return sessions.map((session) => ({
      ...session,
      isFavorite: isFavorite(session.id),
    }))
  }, [sessions, favorites])

  // 应用过滤条件
  const filteredSessions = useMemo(() => {
    let result = sessionsWithFavorites

    // 收藏过滤
    if (filter.showFavoritesOnly) {
      result = result.filter((s) => s.isFavorite)
    }

    // 时间过滤（仅在非收藏模式时应用）
    if (!filter.showFavoritesOnly && filter.timeRange) {
      result = filterByTimeRange(result, filter.timeRange)
    }

    // 搜索过滤
    if (filter.searchQuery) {
      result = searchSessions(result, filter.searchQuery)
    }

    return result
  }, [sessionsWithFavorites, filter])

  return {
    sessions: filteredSessions,
    allSessions: sessionsWithFavorites,
    loading,
    error,
    filter,
    setFilter,
    toggleFavorite,
    refresh: loadSessions,
  }
}
```

- [ ] **Step 3: 创建 hooks 入口**

创建 `src/hooks/index.ts`：

```typescript
export { useSessions } from './useSessions'
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建 useSessions hook"
```

---

## Phase 2 完成检查

- [ ] **验证数据层功能**

创建临时测试组件验证数据加载：

编辑 `src/App.tsx`：

```typescript
import { AppLayout } from "@/components/layout"
import { useSessions } from "@/hooks"
import { useEffect } from "react"

function App() {
  const { sessions, loading, error } = useSessions()

  useEffect(() => {
    console.log("Sessions:", sessions)
  }, [sessions])

  return (
    <AppLayout>
      <div className="flex items-center justify-center h-full text-muted-foreground">
        {loading && "加载中..."}
        {error && `错误: ${error}`}
        {!loading && !error && `已加载 ${sessions.length} 个 session`}
      </div>
    </AppLayout>
  )
}

export default App
```

```bash
npm run tauri dev
```

Expected: 应用显示已加载的 session 数量

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 2 Session 数据层完成"
```