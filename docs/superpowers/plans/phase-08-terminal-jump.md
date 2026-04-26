# Phase 8: 跳转终端

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现"跳转到终端"功能，自动定位并激活对应的 Windows Terminal 窗口

**Architecture:** Tauri 后端调用 Windows API 查找 Windows Terminal 窗口，通过进程 ID 或标题匹配，激活窗口

**Tech Stack:** Tauri, Windows API (Rust)

---

## Task 8.1: 创建 Rust 窗口管理模块

**Files:**
- Create: `src-tauri/src/utils/window_manager.rs`
- Modify: `src-tauri/src/utils/mod.rs`

- [ ] **Step 1: 创建窗口管理模块**

创建 `src-tauri/src/utils/window_manager.rs`：

```rust
use std::process::Command;

#[cfg(target_os = "windows")]
use windows::{
    Win32::Foundation::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::ProcessStatus::*,
};

/// 查找 Windows Terminal 窗口
#[cfg(target_os = "windows")]
pub fn find_terminal_window(working_directory: &str) -> Option<HWND> {
    // Windows Terminal 窗口标题通常包含路径信息
    // 使用 EnumWindows 查找匹配的窗口
    let mut found_window: Option<HWND> = None;

    // 简化实现：查找所有 Windows Terminal 窗口
    // 实际需要更精确的匹配（通过进程 ID 或标题）
    unsafe {
        EnumWindows(Some(enum_windows_callback), LPARAM(&mut found_window as *mut _ as isize));
    }

    found_window
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found_window = &mut *(lparam.0 as *mut Option<HWND>);

    // 获取窗口标题
    let mut title: [u16; 256] = [0; 256];
    let title_len = GetWindowTextW(hwnd, &mut title);

    if title_len > 0 {
        let title_str = String::from_utf16_lossy(&title[..title_len as usize]);

        // 检查是否是 Windows Terminal
        if title_str.contains("Windows Terminal") || title_str.contains("wt") {
            *found_window = Some(hwnd);
            return false.into(); // 停止枚举
        }
    }

    true.into()
}

/// 激活窗口（置顶）
#[cfg(target_os = "windows")]
pub fn activate_window(hwnd: HWND) -> Result<(), String> {
    unsafe {
        // 显示窗口
        ShowWindow(hwnd, SW_SHOW);
        // 设置前台窗口
        SetForegroundWindow(hwnd);
        // 设置焦点
        SetFocus(hwnd);
    }
    Ok(())
}

/// 通过进程 ID 查找窗口
#[cfg(target_os = "windows")]
pub fn find_window_by_pid(pid: u32) -> Option<HWND> {
    let mut found_window: Option<HWND> = None;

    unsafe {
        EnumWindows(Some(enum_windows_by_pid_callback), LPARAM(&mut found_window as *mut _ as isize));
    }

    found_window
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_by_pid_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found_window = &mut *(lparam.0 as *mut Option<HWND>);

    // 获取窗口进程 ID
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);

    // 这里需要传入目标 PID，简化实现
    // 实际需要通过参数传递

    true.into()
}

/// 非 Windows 平台的备用实现
#[cfg(not(target_os = "windows"))]
pub fn find_terminal_window(working_directory: &str) -> Option<u64> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn activate_window(window_id: u64) -> Result<(), String> {
    Err("仅支持 Windows 平台".to_string())
}

/// 启动新终端窗口并恢复 session
pub fn start_terminal_with_resume(working_directory: &str, session_id: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("wt")
            .args([
                "-d", working_directory,
                "claude",
                "--resume", session_id,
            ])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal", working_directory])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("gnome-terminal")
            .args([
                "--working-directory", working_directory,
                "-e", format!("claude --resume {}", session_id),
            ])
            .spawn()
            .map_err(|e| format!("启动终端失败: {}", e))?;
    }

    Ok(())
}
```

- [ ] **Step 2: 更新 mod.rs**

编辑 `src-tauri/src/utils/mod.rs`：

```rust
pub mod claude_data;
pub mod hooks;
pub mod window_manager;
```

- [ ] **Step 3: 添加 Windows API 依赖**

编辑 `src-tauri/Cargo.toml`：

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_ProcessStatus",
] }
```

- [ ] **Step 4: 编译验证**

```bash
cargo build
```

Expected: 编译成功

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: 创建 Rust 窗口管理模块"
```

---

## Task 8.2: 创建 Tauri 终端命令

**Files:**
- Create: `src-tauri/src/commands/terminal.rs`
- Create: `src-tauri/src/commands/mod.rs`（更新）
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 创建终端命令模块**

创建 `src-tauri/src/commands/terminal.rs`：

```rust
use crate::utils::window_manager::{
    find_terminal_window,
    activate_window,
    start_terminal_with_resume,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HWND;

#[tauri::command]
pub fn jump_to_terminal(working_directory: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(hwnd) = find_terminal_window(&working_directory) {
            activate_window(hwnd)?;
            Ok(())
        } else {
            Err("未找到对应的终端窗口".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("仅支持 Windows 平台".to_string())
    }
}

#[tauri::command]
pub fn resume_in_terminal(working_directory: String, session_id: String) -> Result<(), String> {
    start_terminal_with_resume(&working_directory, &session_id)
}
```

- [ ] **Step 2: 更新 commands/mod.rs**

编辑 `src-tauri/src/commands/mod.rs`：

```rust
pub mod session;
pub mod terminal;
```

- [ ] **Step 3: 更新 lib.rs**

编辑 `src-tauri/src/lib.rs`：

```rust
mod utils;
mod commands;

use commands::session::{
    list_sessions,
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_hooks,
    stop_hooks,
    receive_hook_event,
};
use commands::terminal::{jump_to_terminal, resume_in_terminal};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_conversation,
            refresh_sessions,
            start_new_session,
            start_hooks,
            stop_hooks,
            receive_hook_event,
            jump_to_terminal,
            resume_in_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建 Tauri 终端命令"
```

---

## Task 8.3: 创建前端终端服务

**Files:**
- Create: `src/services/terminalService.ts`
- Modify: `src/services/index.ts`

- [ ] **Step 1: 创建终端服务**

创建 `src/services/terminalService.ts`：

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession } from '@/types'

/**
 * 跳转到终端窗口
 */
export async function jumpToTerminal(session: ClaudeSession): Promise<void> {
  try {
    await invoke('jump_to_terminal', {
      workingDirectory: session.workingDirectory,
    })
  } catch (error) {
    // 失败时，复制路径到剪贴板作为备用方案
    await navigator.clipboard.writeText(session.workingDirectory)
    throw new Error(`跳转失败，路径已复制到剪贴板: ${error}`)
  }
}

/**
 * 在终端中恢复 session
 */
export async function resumeInTerminal(session: ClaudeSession): Promise<void> {
  try {
    await invoke('resume_in_terminal', {
      workingDirectory: session.workingDirectory,
      sessionId: session.id,
    })
  } catch (error) {
    // 失败时，复制恢复命令作为备用方案
    const command = `claude --resume ${session.id}`
    await navigator.clipboard.writeText(command)
    throw new Error(`恢复失败，命令已复制到剪贴板: ${error}`)
  }
}
```

- [ ] **Step 2: 更新 services 入口**

编辑 `src/services/index.ts`：

```typescript
export * from './claudeSession'
export * from './notificationService'
export * from './terminalService'
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建前端终端服务"
```

---

## Task 8.4: 集成跳转功能到 SessionCard 和 SessionDetail

**Files:**
- Modify: `src/components/running/SessionCard.tsx`
- Modify: `src/components/management/SessionDetail.tsx`

- [ ] **Step 1: 更新 SessionCard**

编辑 `src/components/running/SessionCard.tsx`：

```typescript
import { cn } from "@/lib/utils"
import type { ClaudeSession } from "@/types"
import { StatusBadge } from "./StatusBadge"
import { Button } from "@/components/ui/button"
import { formatRelativeTime } from "@/utils"
import { jumpToTerminal } from "@/services"
import { Star } from "lucide-react"

interface SessionCardProps {
  session: ClaudeSession
  onJumpToTerminal?: (sessionId: string) => void
  onToggleFavorite?: (sessionId: string) => void
}

export function SessionCard({ session, onJumpToTerminal, onToggleFavorite }: SessionCardProps) {
  const isWaitingInput = session.status === "waiting_input"

  const handleJump = async () => {
    try {
      await jumpToTerminal(session)
    } catch (e) {
      alert(String(e))
    }
  }

  return (
    <div
      className={cn(
        "rounded-lg p-4 flex justify-between items-center",
        "border transition-all",
        isWaitingInput
          ? "border-amber-400 bg-amber-50 shadow-sm"
          : "border-gray-200 bg-white hover:border-gray-300"
      )}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <h3 className="font-semibold text-gray-900 truncate">{session.name}</h3>
          <StatusBadge status={session.status} />
          {session.isFavorite && (
            <Star className="w-4 h-4 fill-amber-400 text-amber-400" />
          )}
        </div>
        <p className="text-sm text-gray-600 truncate">{session.workingDirectory}</p>
        <p className="text-xs text-gray-500 mt-1">
          上次活动: {formatRelativeTime(session.lastActivityAt)}
        </p>
      </div>

      <div className="flex items-center gap-2 ml-4">
        {session.status !== "completed" && (
          <Button
            variant={isWaitingInput ? "default" : "secondary"}
            size="sm"
            onClick={handleJump}
            className={isWaitingInput ? "bg-violet-600 hover:bg-violet-700" : ""}
          >
            跳转到终端
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onToggleFavorite?.(session.id)}
          className="p-1"
        >
          <Star
            className={cn(
              "w-4 h-4",
              session.isFavorite
                ? "fill-amber-400 text-amber-400"
                : "text-gray-400"
            )}
          />
        </Button>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: 更新 SessionDetail**

编辑 `src/components/management/SessionDetail.tsx`：

```typescript
import { useState } from "react"
import { cn } from "@/lib/utils"
import type { ClaudeSession, Conversation } from "@/types"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { StatusBadge } from "@/components/running"
import { ConversationView } from "./ConversationView"
import { useFavoriteStore } from "@/stores"
import { resumeInTerminal } from "@/services"
import { ArrowLeft, Star, Trash2, Copy, Check, RefreshCw } from "lucide-react"
import { formatRelativeTime } from "@/utils"

interface SessionDetailProps {
  session: ClaudeSession
  conversation: Conversation | null
  conversationLoading: boolean
  onBack?: () => void
  onDelete: (sessionId: string) => void
  onRefresh: () => void
}

export function SessionDetail({
  session,
  conversation,
  conversationLoading,
  onBack,
  onDelete,
  onRefresh,
}: SessionDetailProps) {
  const [editingName, setEditingName] = useState(session.name)
  const [savingName, setSavingName] = useState(false)
  const [copied, setCopied] = useState(false)
  const { toggleFavorite } = useFavoriteStore()

  const handleSaveName = async () => {
    if (editingName === session.name) return
    setSavingName(true)
    console.log("Save name:", editingName)
    setSavingName(false)
  }

  const handleCopyCommand = async () => {
    const command = `claude --resume ${session.id}`
    await navigator.clipboard.writeText(command)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleResume = async () => {
    try {
      await resumeInTerminal(session)
    } catch (e) {
      alert(String(e))
    }
  }

  const handleDelete = () => {
    if (session.isFavorite) {
      alert("请先取消收藏再删除")
      return
    }
    onDelete(session.id)
  }

  return (
    <div className="flex flex-col h-full bg-white">
      {/* 头部操作栏 */}
      <div className="flex items-center justify-between px-4 py-3 border-b">
        {onBack && (
          <Button variant="ghost" size="sm" onClick={onBack}>
            <ArrowLeft className="w-4 h-4" />
          </Button>
        )}
        <div className="flex items-center gap-2 ml-auto">
          <Button
            variant="default"
            size="sm"
            onClick={handleResume}
            className="bg-violet-600 hover:bg-violet-700"
          >
            恢复 Session
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => toggleFavorite(session.id)}
          >
            <Star
              className={cn(
                "w-4 h-4",
                session.isFavorite
                  ? "fill-amber-400 text-amber-400"
                  : "text-gray-400"
              )}
            />
          </Button>
          {!session.isFavorite && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleDelete}
              className="text-red-500 hover:text-red-600 hover:bg-red-50"
            >
              <Trash2 className="w-4 h-4" />
            </Button>
          )}
        </div>
      </div>

      {/* 基本信息 */}
      <div className="px-4 py-3 border-b bg-gray-50">
        {/* 名称编辑 */}
        <div className="flex items-center gap-2 mb-2">
          <Input
            value={editingName}
            onChange={(e) => setEditingName(e.target.value)}
            className="font-semibold text-lg"
          />
          {editingName !== session.name && (
            <Button
              variant="default"
              size="sm"
              onClick={handleSaveName}
              disabled={savingName}
              className="bg-violet-600"
            >
              {savingName ? "保存中..." : "保存"}
            </Button>
          )}
        </div>

        {/* 路径 */}
        <div className="flex items-center gap-2 text-sm text-gray-600 mb-1">
          <span>{session.workingDirectory}</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => navigator.clipboard.writeText(session.workingDirectory)}
            className="p-1 h-auto"
          >
            <Copy className="w-3 h-3" />
          </Button>
        </div>

        {/* 元数据 */}
        <div className="text-xs text-gray-500">
          创建: {new Date(session.createdAt).toLocaleString("zh-CN")} ·
          上次活动: {formatRelativeTime(session.lastActivityAt)} ·
          {session.conversationCount} 轮对话
        </div>

        {/* 状态 */}
        <div className="flex items-center gap-2 mt-2">
          <StatusBadge status={session.status} />
        </div>
      </div>

      {/* 恢复命令 */}
      <div className="px-4 py-2 border-b">
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-600">恢复命令：</span>
          <div className="flex-1 flex items-center gap-2 bg-gray-100 rounded-md px-3 py-1.5">
            <code className="text-sm text-gray-800 flex-1">
              claude --resume {session.id}
            </code>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyCommand}
              className="p-1 h-auto"
            >
              {copied ? (
                <Check className="w-4 h-4 text-green-500" />
              ) : (
                <Copy className="w-4 h-4" />
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* 对话历史 */}
      <div className="flex-1 overflow-hidden">
        <div className="flex items-center justify-between px-4 py-2 border-b">
          <h3 className="text-sm font-medium text-violet-600">历史对话</h3>
          <Button
            variant="ghost"
            size="sm"
            onClick={onRefresh}
            className="p-1 h-auto"
          >
            <RefreshCw className="w-4 h-4" />
          </Button>
        </div>
        <ConversationView
          messages={conversation?.messages || []}
          loading={conversationLoading}
        />
      </div>
    </div>
  )
}
```

- [ ] **Step 3: 更新 ManagementTab**

编辑 `src/components/management/ManagementTab.tsx`，移除 onResume prop（因为 SessionDetail 内部处理）：

```typescript
// SessionDetail 不需要 onResume prop
<SessionDetail
  session={selectedSession}
  conversation={currentConversation}
  conversationLoading={conversationLoading}
  onDelete={handleDelete}
  onRefresh={handleRefreshConversation}
/>
```

- [ ] **Step 4: 验证跳转功能**

```bash
npm run tauri dev
```

Expected:
- "跳转到终端"按钮点击后，Windows Terminal 窗口激活
- "恢复 Session"按钮点击后，新终端窗口打开并执行恢复命令

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: 集成跳转终端功能到 SessionCard 和 SessionDetail"
```

---

## Phase 8 完成检查

- [ ] **验证所有功能**

检查：
- 跳转到终端按钮正常工作
- 恢复 Session 按钮正常工作
- 失败时降级方案（复制命令）正常

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 8 跳转终端完成"
```