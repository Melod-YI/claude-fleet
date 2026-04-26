# Phase 6: 新建 Session

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现新建 Session 弹窗，包括路径选择、名称输入、启动 Claude Code

**Architecture:** 弹窗组件（Dialog），包含路径输入/浏览/快捷选择，启动按钮调用 Tauri 命令

**Tech Stack:** React, TypeScript, Tailwind CSS, shadcn/ui, Tauri Shell

---

## Task 6.1: 创建 NewSessionDialog 组件

**Files:**
- Create: `src/components/dialogs/NewSessionDialog.tsx`
- Create: `src/components/dialogs/index.ts`

- [ ] **Step 1: 创建 dialogs 目录**

```bash
mkdir -p src/components/dialogs
```

- [ ] **Step 2: 创建新建 Session 弹窗组件**

创建 `src/components/dialogs/NewSessionDialog.tsx`：

```typescript
import { useState } from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { cn } from "@/lib/utils"
import { FolderOpen, Loader2 } from "lucide-react"

interface NewSessionDialogProps {
  open: boolean
  onClose: () => void
  favoritePaths: string[]
  onAddFavoritePath: (path: string) => void
}

export function NewSessionDialog({
  open,
  onClose,
  favoritePaths,
  onAddFavoritePath,
}: NewSessionDialogProps) {
  const [workingDirectory, setWorkingDirectory] = useState("")
  const [sessionName, setSessionName] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleBrowse = async () => {
    // 调用 Tauri 打开文件夹选择对话框
    try {
      const selectedPath = await window.__TAURI__.dialog.open({
        directory: true,
        multiple: false,
      })
      if (selectedPath) {
        setWorkingDirectory(selectedPath as string)
        setSessionName("") // 清空名称，使用默认
      }
    } catch (e) {
      console.error("打开文件夹选择失败:", e)
    }
  }

  const handleSelectFavoritePath = (path: string) => {
    setWorkingDirectory(path)
    setSessionName("")
  }

  const handleStart = async () => {
    if (!workingDirectory.trim()) {
      setError("请选择工作目录")
      return
    }

    setLoading(true)
    setError(null)

    try {
      // 调用 Tauri 命令启动 Claude Code
      await window.__TAURI__.invoke('start_new_session', {
        workingDirectory,
        name: sessionName || undefined,
      })

      // 新建的 session 默认加入收藏
      // 需要在后端返回 session ID 后添加
      // 暂时先关闭弹窗
      onClose()

      // 刷新 session 列表
      // Phase 7 实现钩子通知后会自动更新
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>新建 Session</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-4">
          {/* 工作目录 */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium text-gray-700">工作目录</label>
            <div className="flex gap-2">
              <Input
                value={workingDirectory}
                onChange={(e) => setWorkingDirectory(e.target.value)}
                placeholder="选择或输入路径..."
                className="flex-1"
              />
              <Button
                variant="outline"
                size="sm"
                onClick={handleBrowse}
              >
                <FolderOpen className="w-4 h-4" />
                浏览...
              </Button>
            </div>
          </div>

          {/* Session 名称 */}
          <div className="flex flex-col gap-2">
            <label className="text-sm font-medium text-gray-700">
              Session 名称（可选）
            </label>
            <Input
              value={sessionName}
              onChange={(e) => setSessionName(e.target.value)}
              placeholder="默认使用目录名称"
            />
          </div>

          {/* 常用路径 */}
          {favoritePaths.length > 0 && (
            <div className="flex flex-col gap-2">
              <label className="text-sm font-medium text-gray-700">常用路径</label>
              <div className="flex flex-wrap gap-2">
                {favoritePaths.map((path) => (
                  <Button
                    key={path}
                    variant="outline"
                    size="sm"
                    onClick={() => handleSelectFavoritePath(path)}
                    className={cn(
                      "text-xs",
                      workingDirectory === path && "border-violet-500 bg-violet-50"
                    )}
                  >
                    {path}
                  </Button>
                ))}
              </div>
            </div>
          )}

          {/* 错误提示 */}
          {error && (
            <div className="text-sm text-red-500">{error}</div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button
            variant="default"
            onClick={handleStart}
            disabled={loading}
            className="bg-violet-600 hover:bg-violet-700"
          >
            {loading ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                启动中...
              </>
            ) : (
              "启动 Claude Code"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 3: 创建 dialogs 入口**

创建 `src/components/dialogs/index.ts`：

```typescript
export { NewSessionDialog } from './NewSessionDialog'
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建 NewSessionDialog 弹窗组件"
```

---

## Task 6.2: 创建 Tauri 启动 Session 命令

**Files:**
- Create: `src-tauri/src/commands/session.rs`（添加命令）
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加启动 session 命令**

编辑 `src-tauri/src/commands/session.rs`，添加新命令：

```rust
use crate::utils::claude_data::{get_all_sessions, get_session_conversation, ClaudeSession, Conversation};
use tauri::Manager;

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

#[tauri::command]
pub async fn start_new_session(
    app: tauri::AppHandle,
    working_directory: String,
    name: Option<String>,
) -> Result<String, String> {
    // 使用 shell plugin 启动 Windows Terminal
    use tauri_plugin_shell::ShellExt;

    let terminal_cmd = if cfg!(target_os = "windows") {
        // Windows: 使用 wt (Windows Terminal)
        format!("wt -d \"{}\" claude", working_directory)
    } else if cfg!(target_os = "macos") {
        // macOS: 使用 open 命令打开 Terminal
        format!("open -a Terminal \"{}\"", working_directory)
    } else {
        // Linux: 使用 gnome-terminal
        format!("gnome-terminal --working-directory=\"{}\" -e claude", working_directory)
    };

    // 执行命令
    let shell = app.shell();
    let result = shell
        .command("sh")
        .args(["-c", &terminal_cmd])
        .output();

    match result {
        Ok(_) => Ok(format!("已在 {} 启动 Claude Code", working_directory)),
        Err(e) => Err(format!("启动失败: {}", e)),
    }
}
```

- [ ] **Step 2: 更新 lib.rs 注册新命令**

编辑 `src-tauri/src/lib.rs`：

```rust
mod utils;
mod commands;

use commands::session::{list_sessions, get_conversation, refresh_sessions, start_new_session};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_conversation,
            refresh_sessions,
            start_new_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 添加 shell plugin 权限**

在 `src-tauri/capabilities/default.json` 中添加 shell 权限（如果文件不存在则创建）：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open",
    "shell:allow-execute",
    "dialog:allow-open"
  ]
}
```

- [ ] **Step 4: 安装 dialog plugin**

编辑 `src-tauri/Cargo.toml`：

```toml
[dependencies]
tauri = { version = "2", features = ["notification-all", "shell-open"] }
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"
```

- [ ] **Step 5: 更新 lib.rs 添加 dialog plugin**

编辑 `src-tauri/src/lib.rs`：

```rust
mod utils;
mod commands;

use commands::session::{list_sessions, get_conversation, refresh_sessions, start_new_session};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_conversation,
            refresh_sessions,
            start_new_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: 编译验证**

```bash
cargo build
```

Expected: 编译成功

- [ ] **Step 7: Commit**

```bash
git add .
git commit -m "feat: 创建 Tauri start_new_session 命令"
```

---

## Task 6.3: 创建 Settings Store 管理常用路径

**Files:**
- Create: `src/stores/settingsStore.ts`
- Modify: `src/stores/index.ts`

- [ ] **Step 1: 创建 settings store**

创建 `src/stores/settingsStore.ts`：

```typescript
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
```

- [ ] **Step 2: 更新 stores 入口**

编辑 `src/stores/index.ts`：

```typescript
export { useSessionStore } from './sessionStore'
export { useFavoriteStore } from './favoriteStore'
export { useSettingsStore } from './settingsStore'
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建 settings store 管理常用路径和设置"
```

---

## Task 6.4: 集成 NewSessionDialog 到 ManagementTab

**Files:**
- Modify: `src/components/management/ManagementTab.tsx`

- [ ] **Step 1: 更新 ManagementTab**

编辑 `src/components/management/ManagementTab.tsx`：

```typescript
import { useState } from "react"
import { SplitPane } from "@/components/layout"
import { SessionList } from "./SessionList"
import { SessionDetail } from "./SessionDetail"
import { NewSessionDialog } from "@/components/dialogs"
import { useSessionStore, useSettingsStore, useFavoriteStore } from "@/stores"
import type { ClaudeSession } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<ClaudeSession | null>(null)
  const { currentConversation, selectSession, loading: conversationLoading } = useSessionStore()
  const { favoritePaths, addFavoritePath } = useSettingsStore()
  const { addFavorite } = useFavoriteStore()
  const [showNewSessionDialog, setShowNewSessionDialog] = useState(false)

  const handleSelectSession = async (session: ClaudeSession) => {
    setSelectedSession(session)
    await selectSession(session.id)
  }

  const handleNewSession = () => {
    setShowNewSessionDialog(true)
  }

  const handleCloseNewSessionDialog = () => {
    setShowNewSessionDialog(false)
  }

  const handleResume = (sessionId: string) => {
    // Phase 6 实现
    console.log("Resume session:", sessionId)
  }

  const handleDelete = (sessionId: string) => {
    // Phase 6 实现
    console.log("Delete session:", sessionId)
  }

  const handleRefreshConversation = async () => {
    if (selectedSession) {
      await selectSession(selectedSession.id)
    }
  }

  return (
    <>
      <SplitPane
        left={
          <SessionList
            selectedSessionId={selectedSession?.id || null}
            onSelectSession={handleSelectSession}
            onNewSession={handleNewSession}
          />
        }
        right={
          selectedSession ? (
            <SessionDetail
              session={selectedSession}
              conversation={currentConversation}
              conversationLoading={conversationLoading}
              onResume={handleResume}
              onDelete={handleDelete}
              onRefresh={handleRefreshConversation}
            />
          ) : (
            <div className="flex items-center justify-center h-full text-gray-500">
              请从左侧列表选择一个 session
            </div>
          )
        }
        leftWidth={280}
      />

      <NewSessionDialog
        open={showNewSessionDialog}
        onClose={handleCloseNewSessionDialog}
        favoritePaths={favoritePaths.paths}
        onAddFavoritePath={addFavoritePath}
      />
    </>
  )
}
```

- [ ] **Step 2: 验证新建 Session 功能**

```bash
npm run tauri dev
```

Expected:
- 点击 "+" 按钮，弹窗显示
- 浏览按钮可打开文件夹选择
- 常用路径按钮可点击
- 启动按钮可点击（会打开 Windows Terminal）

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 集成 NewSessionDialog 到 ManagementTab"
```

---

## Phase 6 完成检查

- [ ] **验证所有功能**

检查：
- 弹窗正常显示和关闭
- 浏览按钮打开文件夹选择对话框
- 常用路径按钮可选择
- 启动按钮启动 Windows Terminal
- 新建的 session 自动加入收藏

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 6 新建 Session 完成"
```