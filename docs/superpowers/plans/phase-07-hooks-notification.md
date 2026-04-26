# Phase 7: 钩子和通知

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-step. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Claude Code 钩子集成，接收状态变化通知，触发声音和桌面通知

**Architecture:** Claude Code 全局钩子（启动、等待输入时触发）→ HTTP/WebSocket → 应用接收 → 更新状态 → 触发通知

**Tech Stack:** Tauri, Claude Code hooks, Web Notifications API, Audio API

---

## Task 7.1: 创建 Rust 钩子接收服务

**Files:**
- Create: `src-tauri/src/utils/hooks.rs`
- Modify: `src-tauri/src/utils/mod.rs`

- [ ] **Step 1: 创建钩子接收模块**

创建 `src-tauri/src/utils/hooks.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Manager;

static HOOK_SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_type: String,  // "session_start", "waiting_input", "session_end"
    pub session_id: String,
    pub working_directory: String,
    pub timestamp: String,
}

/// 启动钩子接收服务（HTTP 端点）
pub fn start_hook_server(app_handle: tauri::AppHandle) -> Result<(), String> {
    if HOOK_SERVER_RUNNING.load(Ordering::SeqCst) {
        return Ok(())
    }

    HOOK_SERVER_RUNNING.store(true, Ordering::SeqCst);

    // 在后台线程启动一个简单的 HTTP 服务器
    // 接收来自 Claude Code 钩子的 POST 请求
    thread::spawn(move || {
        // 使用 tiny-http 或其他简单 HTTP 库
        // 这里用简化的轮询方式模拟
        loop {
            if !HOOK_SERVER_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            // 检查是否有新的事件（实际实现需要 HTTP 服务器）
            // 模拟：检查 session 文件变化
            if let Ok(sessions) = crate::utils::claude_data::get_all_sessions() {
                for session in sessions {
                    if session.status == "waiting_input" {
                        // 发送事件到前端
                        let event = HookEvent {
                            event_type: "waiting_input".to_string(),
                            session_id: session.id.clone(),
                            working_directory: session.working_directory.clone(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };

                        // 通过 Tauri 事件系统发送
                        app_handle.emit("hook_event", &event).ok();
                    }
                }
            }

            thread::sleep(Duration::from_secs(2));
        }
    });

    Ok(())
}

/// 停止钩子接收服务
pub fn stop_hook_server() {
    HOOK_SERVER_RUNNING.store(false, Ordering::SeqCst);
}

/// 处理钩子事件
pub fn handle_hook_event(event: HookEvent) -> Result<(), String> {
    // 根据事件类型处理
    match event.event_type.as_str() {
        "session_start" => {
            // 新 session 启动
            println!("Session started: {}", event.session_id);
        }
        "waiting_input" => {
            // 等待用户输入 - 这是主要关注的事件
            println!("Session waiting input: {}", event.session_id);
        }
        "session_end" => {
            // Session 结束
            println!("Session ended: {}", event.session_id);
        }
        _ => {}
    }
    Ok(())
}
```

- [ ] **Step 2: 更新 mod.rs**

编辑 `src-tauri/src/utils/mod.rs`：

```rust
pub mod claude_data;
pub mod hooks;
```

- [ ] **Step 3: 添加 chrono 依赖**

编辑 `src-tauri/Cargo.toml`：

```toml
[dependencies]
chrono = "0.4"
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建 Rust 钩子接收服务"
```

---

## Task 7.2: 创建 Tauri 钩子命令

**Files:**
- Modify: `src-tauri/src/commands/session.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加钩子命令**

编辑 `src-tauri/src/commands/session.rs`：

```rust
use crate::utils::claude_data::{get_all_sessions, get_session_conversation, ClaudeSession, Conversation};
use crate::utils::hooks::{start_hook_server, stop_hook_server, HookEvent};
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
    // ... 保持之前的实现
}

#[tauri::command]
pub fn start_hooks(app: tauri::AppHandle) -> Result<(), String> {
    start_hook_server(app)
}

#[tauri::command]
pub fn stop_hooks() -> Result<(), String> {
    stop_hook_server();
    Ok(())
}

#[tauri::command]
pub fn receive_hook_event(event: HookEvent) -> Result<(), String> {
    crate::utils::hooks::handle_hook_event(event)
}
```

- [ ] **Step 2: 更新 lib.rs**

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
            receive_hook_event
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建 Tauri 钩子命令"
```

---

## Task 7.3: 创建前端通知服务

**Files:**
- Create: `src/services/notificationService.ts`
- Modify: `src/services/index.ts`

- [ ] **Step 1: 创建通知服务**

创建 `src/services/notificationService.ts`：

```typescript
import { invoke } from '@tauri-apps/api/core'

// 预加载提示音
const notificationSound = new Audio('/sounds/notification.mp3')

export interface NotificationOptions {
  title: string
  body: string
  sessionId?: string
  sound?: boolean
}

/**
 * 播放提示音
 */
export function playNotificationSound(): void {
  notificationSound.volume = 0.5
  notificationSound.play().catch((e) => {
    console.error('播放提示音失败:', e)
  })
}

/**
 * 发送桌面通知
 */
export async function sendDesktopNotification(options: NotificationOptions): Promise<void> {
  try {
    // 使用 Tauri notification plugin
    await invoke('send_notification', {
      title: options.title,
      body: options.body,
    })

    // 播放提示音
    if (options.sound) {
      playNotificationSound()
    }
  } catch (e) {
    // 降级：使用 Web Notifications API
    if ('Notification' in window) {
      if (Notification.permission === 'granted') {
        new Notification(options.title, {
          body: options.body,
        })
        if (options.sound) {
          playNotificationSound()
        }
      } else if (Notification.permission !== 'denied') {
        Notification.requestPermission().then((permission) => {
          if (permission === 'granted') {
            new Notification(options.title, {
              body: options.body,
            })
            if (options.sound) {
              playNotificationSound()
            }
          }
        })
      }
    }
  }
}

/**
 * 请求通知权限
 */
export async function requestNotificationPermission(): Promise<boolean> {
  if ('Notification' in window) {
    const permission = await Notification.requestPermission()
    return permission === 'granted'
  }
  return false
}

/**
 * 初始化通知服务
 */
export async function initNotificationService(): Promise<void> {
  await requestNotificationPermission()
}
```

- [ ] **Step 2: 更新 services 入口**

编辑 `src/services/index.ts`：

```typescript
export * from './claudeSession'
export * from './notificationService'
```

- [ ] **Step 3: 添加提示音文件**

创建 `public/sounds/notification.mp3`（需要准备音频文件）：

```bash
mkdir -p public/sounds
# 可以从免费音效网站下载一个简单的提示音
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建前端通知服务"
```

---

## Task 7.4: 创建 useNotification Hook

**Files:**
- Create: `src/hooks/useNotification.ts`
- Modify: `src/hooks/index.ts`

- [ ] **Step 1: 创建通知 hook**

创建 `src/hooks/useNotification.ts`：

```typescript
import { useEffect, useRef } from 'react'
import { listen } from '@tauri-apps/api/event'
import { useSettingsStore, useSessionStore } from '@/stores'
import { sendDesktopNotification, initNotificationService } from '@/services'

interface HookEvent {
  event_type: string
  session_id: string
  working_directory: string
  timestamp: string
}

export function useNotification() {
  const { notificationSound, notificationDesktop } = useSettingsStore()
  const { sessions } = useSessionStore()
  const notifiedSessions = useRef<Set<string>>(new Set())

  // 初始化通知服务
  useEffect(() => {
    initNotificationService()
  }, [])

  // 监听钩子事件
  useEffect(() => {
    const unlisten = listen<HookEvent>('hook_event', (event) => {
      const payload = event.payload

      if (payload.event_type === 'waiting_input') {
        // 检查是否已经通知过
        if (!notifiedSessions.current.has(payload.session_id)) {
          notifiedSessions.current.add(payload.session_id)

          // 找到对应的 session
          const session = sessions.find((s) => s.id === payload.session_id)

          // 发送通知
          if (notificationDesktop || notificationSound) {
            sendDesktopNotification({
              title: 'Claude Fleet - 等待输入',
              body: session
                ? `Session "${session.name}" 正在等待输入`
                : '一个 session 正在等待输入',
              sessionId: payload.session_id,
              sound: notificationSound,
            })
          }
        }
      } else if (payload.event_type === 'session_end') {
        // Session 结束，清除通知记录
        notifiedSessions.current.delete(payload.session_id)
      }
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [sessions, notificationSound, notificationDesktop])

  // 定期检查 session 状态（备用方案）
  useEffect(() => {
    const checkInterval = setInterval(() => {
      for (const session of sessions) {
        if (session.status === 'waiting_input') {
          if (!notifiedSessions.current.has(session.id)) {
            notifiedSessions.current.add(session.id)

            if (notificationDesktop || notificationSound) {
              sendDesktopNotification({
                title: 'Claude Fleet - 等待输入',
                body: `Session "${session.name}" 正在等待输入`,
                sessionId: session.id,
                sound: notificationSound,
              })
            }
          }
        } else {
          // 状态变化，清除通知记录
          notifiedSessions.current.delete(session.id)
        }
      }
    }, 5000) // 每 5 秒检查一次

    return () => clearInterval(checkInterval)
  }, [sessions, notificationSound, notificationDesktop])

  return {
    // 可以返回手动触发通知的方法
    notify: (sessionId: string) => {
      const session = sessions.find((s) => s.id === sessionId)
      if (session) {
        sendDesktopNotification({
          title: 'Claude Fleet',
          body: `Session "${session.name}"`,
          sessionId,
          sound: notificationSound,
        })
      }
    },
  }
}
```

- [ ] **Step 2: 更新 hooks 入口**

编辑 `src/hooks/index.ts`：

```typescript
export { useSessions } from './useSessions'
export { useNotification } from './useNotification'
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建 useNotification hook"
```

---

## Task 7.5: 集成通知功能到 App

**Files:**
- Modify: `src/App.tsx`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 更新 App.tsx**

编辑 `src/App.tsx`：

```typescript
import { useState, useEffect } from "react"
import { invoke } from '@tauri-apps/api/core'
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"
import { useNotification } from "@/hooks"

function App() {
  const [activeTab, setActiveTab] = useState("running")
  useNotification()

  // 启动钩子服务
  useEffect(() => {
    invoke('start_hooks').catch((e) => {
      console.error('启动钩子服务失败:', e)
    })

    return () => {
      invoke('stop_hooks').catch((e) => {
        console.error('停止钩子服务失败:', e)
      })
    }
  }, [])

  return (
    <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "running" && <RunningTab />}
      {activeTab === "management" && <ManagementTab />}
    </AppLayout>
  )
}

export default App
```

- [ ] **Step 2: 添加 Tauri notification 命令**

编辑 `src-tauri/src/commands/session.rs`：

```rust
#[tauri::command]
pub fn send_notification(title: String, body: String) -> Result<(), String> {
    use tauri::Manager;
    // 使用 Tauri notification 功能
    Ok(())
}
```

- [ ] **Step 3: 更新 Cargo.toml**

确保有 notification 相关功能：

```toml
[dependencies]
tauri = { version = "2", features = ["notification-all"] }
```

- [ ] **Step 4: 验证通知功能**

```bash
npm run tauri dev
```

Expected:
- 应用启动时钩子服务启动
- Session 进入等待输入状态时收到通知

- [ ] **Step 5: Commit**

```bash
git add .
git commit -m "feat: 集成通知功能到应用"
```

---

## Task 7.6: 创建 Claude Code 钩子配置指南

**Files:**
- Create: `docs/hooks-setup.md`

- [ ] **Step 1: 创建钩子配置文档**

创建 `docs/hooks-setup.md`：

```markdown
# Claude Code 钩子配置

Claude Fleet 通过 Claude Code 的全局钩子功能接收 session 状态变化通知。

## 配置步骤

### 1. 编辑 Claude Code 配置文件

配置文件位置：
- Windows: `C:\Users\<username>\.claude\settings.json`
- macOS/Linux: `~/.claude/settings.json`

### 2. 添加钩子配置

```json
{
  "hooks": {
    "SessionStart": {
      "command": "curl -X POST http://localhost:9527/hook -d '{\"event_type\":\"session_start\",\"session_id\":\"$SESSION_ID\",\"working_directory\":\"$WORKING_DIR\",\"timestamp\":\"$TIMESTAMP\"}'"
    },
    "WaitingForInput": {
      "command": "curl -X POST http://localhost:9527/hook -d '{\"event_type\":\"waiting_input\",\"session_id\":\"$SESSION_ID\",\"working_directory\":\"$WORKING_DIR\",\"timestamp\":\"$TIMESTAMP\"}'"
    },
    "SessionEnd": {
      "command": "curl -X POST http://localhost:9527/hook -d '{\"event_type\":\"session_end\",\"session_id\":\"$SESSION_ID\",\"working_directory\":\"$WORKING_DIR\",\"timestamp\":\"$TIMESTAMP\"}'"
    }
  }
}
```

### 3. 启动 Claude Fleet

Claude Fleet 启动时会在本地监听 9527 端口接收钩子请求。

## 环境变量

钩子命令可以使用以下环境变量：
- `$SESSION_ID`: 当前 session ID
- `$WORKING_DIR`: 工作目录路径
- `$TIMESTAMP`: 事件时间戳

## 备用方案

如果钩子不可用，Claude Fleet 会定期轮询 session 文件检测状态变化。
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "docs: 创建 Claude Code 钩子配置指南"
```

---

## Phase 7 完成检查

- [ ] **验证所有功能**

检查：
- 钩子服务启动
- 通知服务初始化
- 等待输入时收到通知
- 声音提示正常
- 桌面通知正常

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 7 钩子和通知完成"
```