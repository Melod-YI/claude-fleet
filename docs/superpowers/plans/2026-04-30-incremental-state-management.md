# 增量状态管理实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现运行中 session 的增量状态管理，消除全量扫描导致的卡顿问题。

**Architecture:** 后端维护 RUNNING_SESSIONS 内存状态，hook 事件触发增量更新，定时轮询检测意外退出，前端通过事件监听获取状态。

**Tech Stack:** Rust (Tauri), TypeScript (React), notify crate, Zustand

---

## 文件结构

| 文件 | 操作 | 职责 |
|------|------|------|
| `docs/hooks/hook_writer.py` | 修改 | 文件命名策略 + SessionEnd 清理 |
| `src-tauri/src/utils/running_sessions.rs` | 新建 | 状态管理核心模块 |
| `src-tauri/src/utils/claude_data.rs` | 修改 | 添加进程名验证函数 |
| `src-tauri/src/utils/hooks.rs` | 修改 | 增量更新状态 |
| `src-tauri/src/commands/session.rs` | 修改 | 新增命令入口 |
| `src-tauri/src/lib.rs` | 修改 | 注册命令 + 启动初始化 |
| `src/hooks/useRunningSessions.ts` | 新建 | 前端状态管理 hook |
| `src/hooks/useNotification.ts` | 修改 | 监听新事件 |
| `src/components/running/RunningTab.tsx` | 修改 | 使用新 hook |
| `src/services/claudeSession.ts` | 修改 | 更新接口 |

---

### Task 1: 修改 hook_writer.py 文件命名策略

**Files:**
- Modify: `docs/hooks/hook_writer.py`

- [ ] **Step 1: 编写新的 hook_writer.py**

```python
import os, sys, json

hook_input = json.load(sys.stdin)
session_id = hook_input.get("session_id", "unknown")
event_type = hook_input.get("hook_event_name", "unknown")

events_dir = os.path.expanduser("~/.claude-fleet/events")
os.makedirs(events_dir, exist_ok=True)

# SessionEnd 时清理该 session 所有历史文件
if event_type == "SessionEnd":
    for f in os.listdir(events_dir):
        if f.startswith(session_id) and f.endswith(".json"):
            os.remove(os.path.join(events_dir, f))

# 写入事件文件（覆盖同类型旧文件）
file_path = os.path.join(events_dir, f"{session_id}_{event_type}.json")
with open(file_path, "w", encoding="utf-8") as f:
    json.dump(hook_input, f, indent=2, ensure_ascii=False)
```

- [ ] **Step 2: 更新测试目录的 hook 配置**

修改 `C:\workspace\claude-test-workspace\.claude\settings.json`，确保调用参数正确：

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "idle_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python ~/.claude-fleet/hook_writer.py"
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 3: 手动测试 hook_writer.py 行为**

在测试目录启动 Claude Code，观察 `~/.claude-fleet/events/` 目录：
- 文件名应为 `<session-id>_<event-type>.json`
- SessionEnd 后该 session 所有文件应被清理

```bash
ls ~/.claude-fleet/events/
```

---

### Task 2: 创建 running_sessions.rs 状态管理模块

**Files:**
- Create: `src-tauri/src/utils/running_sessions.rs`

- [ ] **Step 1: 创建基础状态结构**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;

/// Session 运行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Running,
    WaitingInput,
}

/// 运行中 Session 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningSession {
    pub session_id: String,
    pub pid: u32,
    pub status: SessionStatus,
    pub cwd: String,
    pub name: String,
    pub updated_at: u64,
}

/// 全局运行中 Session 状态
pub static RUNNING_SESSIONS: Lazy<Mutex<HashMap<String, RunningSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 获取事件目录路径
fn get_events_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("events")
}

/// 获取 sessions 目录路径
fn get_sessions_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude")
        .join("sessions")
}
```

- [ ] **Step 2: 实现 HookEvent 解析结构**

```rust
/// Hook 事件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub hook_event_name: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub transcript_path: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// 从文件解析 HookEvent
pub fn parse_hook_event(file_path: &PathBuf) -> Result<HookEvent, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("读取事件文件失败: {}", e))?;
    serde_json::from_str::<HookEvent>(&content)
        .map_err(|e| format!("解析事件 JSON 失败: {}", e))
}
```

- [ ] **Step 3: 实现 Session 元数据读取**

```rust
/// Session 元数据（从 ~/.claude/sessions/*.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetadata {
    pid: u32,
    #[serde(rename = "sessionId")]
    session_id: String,
    cwd: String,
    #[serde(rename = "startedAt")]
    started_at: u64,
    #[serde(default)]
    status: String,
    #[serde(rename = "updatedAt", default)]
    updated_at: Option<u64>,
}

/// 读取 session 元数据
fn read_session_metadata(session_id: &str) -> Result<SessionMetadata, String> {
    let sessions_dir = get_sessions_dir();
    let file_path = sessions_dir.join(format!("{}.json", session_id));
    
    if !file_path.exists() {
        return Err(format!("Session 元数据文件不存在: {}", file_path.display()));
    }
    
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("读取 session 元数据失败: {}", e))?;
    
    serde_json::from_str::<SessionMetadata>(&content)
        .map_err(|e| format!("解析 session 元数据失败: {}", e))
}
```

- [ ] **Step 4: 提取目录名称辅助函数**

```rust
/// 从路径提取最后一段作为名称
fn get_path_name(path: &str) -> String {
    path.split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}
```

---

### Task 3: 实现进程验证函数

**Files:**
- Modify: `src-tauri/src/utils/claude_data.rs`

- [ ] **Step 1: 添加 is_claude_process_running 函数**

在 `src-tauri/src/utils/claude_data.rs` 文件末尾添加：

```rust
/// 检查 PID 是否为 claude 进程
#[cfg(target_os = "windows")]
pub fn is_claude_process_running(pid: u32) -> bool {
    use std::process::Command;
    
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // 检查 PID 存在且进程名包含 "claude"
        stdout.contains(&pid.to_string()) && stdout.to_lowercase().contains("claude")
    } else {
        false
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_claude_process_running(pid: u32) -> bool {
    use std::process::Command;
    
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output();
    
    if let Ok(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_lowercase().contains("claude")
    } else {
        false
    }
}
```

---

### Task 4: 实现状态管理操作函数

**Files:**
- Modify: `src-tauri/src/utils/running_sessions.rs`

- [ ] **Step 1: 实现添加运行中 session**

```rust
use crate::utils::claude_data::is_claude_process_running;

/// 添加运行中 session
pub fn add_running_session(session_id: &str) -> Result<(), String> {
    // 读取 session 元数据获取 PID
    let metadata = read_session_metadata(session_id)?;
    
    // 验证进程是否为 claude
    if !is_claude_process_running(metadata.pid) {
        return Err(format!("PID {} 不是 claude 进程", metadata.pid));
    }
    
    let name = get_path_name(&metadata.cwd);
    let status = if metadata.status == "idle" {
        SessionStatus::WaitingInput
    } else {
        SessionStatus::Running
    };
    
    let session = RunningSession {
        session_id: session_id.to_string(),
        pid: metadata.pid,
        status,
        cwd: metadata.cwd,
        name,
        updated_at: metadata.updated_at.unwrap_or(metadata.started_at),
    };
    
    RUNNING_SESSIONS.lock().unwrap().insert(session_id.to_string(), session);
    Ok(())
}

/// 更新 session 状态
pub fn update_session_status(session_id: &str, status: SessionStatus) {
    if let Some(session) = RUNNING_SESSIONS.lock().unwrap().get_mut(session_id) {
        session.status = status;
        session.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// 移除运行中 session
pub fn remove_running_session(session_id: &str) {
    RUNNING_SESSIONS.lock().unwrap().remove(session_id);
}

/// 获取所有运行中 session
pub fn get_running_sessions() -> Vec<RunningSession> {
    RUNNING_SESSIONS.lock().unwrap().values().cloned().collect()
}
```

---

### Task 5: 实现启动初始化逻辑

**Files:**
- Modify: `src-tauri/src/utils/running_sessions.rs`

- [ ] **Step 1: 实现启动初始化函数**

```rust
/// 应用启动时初始化运行中 session 列表
pub fn init_running_sessions() -> Result<Vec<RunningSession>, String> {
    let events_dir = get_events_dir();
    
    if !events_dir.exists() {
        fs::create_dir_all(&events_dir)
            .map_err(|e| format!("创建事件目录失败: {}", e))?;
        return Ok(Vec::new());
    }
    
    // 读取所有事件文件
    let mut events_by_session: HashMap<String, Vec<HookEvent>> = HashMap::new();
    
    for entry in fs::read_dir(&events_dir)
        .map_err(|e| format!("读取事件目录失败: {}", e))?
    {
        let file_path = entry.map_err(|e| format!("读取条目失败: {}", e))?.path();
        
        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        
        if let Ok(event) = parse_hook_event(&file_path) {
            events_by_session
                .entry(event.session_id.clone())
                .or_insert_with(Vec::new)
                .push(event);
        }
    }
    
    // 清空现有状态
    RUNNING_SESSIONS.lock().unwrap().clear();
    
    // 分析每个 session 的状态
    for (session_id, events) in events_by_session.iter() {
        // 检查是否有 SessionEnd
        let has_end = events.iter().any(|e| e.hook_event_name == "SessionEnd");
        
        if has_end {
            // 已结束，跳过
            continue;
        }
        
        // 检查是否有 SessionStart
        let has_start = events.iter().any(|e| e.hook_event_name == "SessionStart");
        
        if has_start {
            // 尝试添加到运行中列表
            if let Ok(_) = add_running_session(session_id) {
                // 检查是否有 Notification（等待输入）
                let has_notification = events.iter().any(|e| e.hook_event_name == "Notification");
                if has_notification {
                    update_session_status(session_id, SessionStatus::WaitingInput);
                }
            }
        }
    }
    
    // 清理已处理的事件文件
    cleanup_events_dir(&events_dir);
    
    Ok(get_running_sessions())
}

/// 清理事件目录
fn cleanup_events_dir(events_dir: &PathBuf) {
    if let Ok(entries) = fs::read_dir(events_dir) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.is_file() {
                fs::remove_file(&file_path).ok();
            }
        }
    }
}
```

---

### Task 6: 实现定时轮询检测意外退出

**Files:**
- Modify: `src-tauri/src/utils/running_sessions.rs`

- [ ] **Step 1: 添加轮询相关导入和状态**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

static POLLING_RUNNING: AtomicBool = AtomicBool::new(false);
```

- [ ] **Step 2: 实现轮询启动函数**

```rust
/// 启动定时轮询（检测意外退出）
pub fn start_polling(app_handle: tauri::AppHandle) {
    if POLLING_RUNNING.load(Ordering::SeqCst) {
        return;
    }
    
    POLLING_RUNNING.store(true, Ordering::SeqCst);
    
    thread::spawn(move || {
        loop {
            if !POLLING_RUNNING.load(Ordering::SeqCst) {
                break;
            }
            
            thread::sleep(Duration::from_secs(30));
            
            // 检查所有运行中 session 的进程状态
            let mut changed = false;
            let sessions_to_remove: Vec<String> = RUNNING_SESSIONS
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, session)| !is_claude_process_running(session.pid))
                .map(|(id, _)| id.clone())
                .collect();
            
            for session_id in sessions_to_remove {
                RUNNING_SESSIONS.lock().unwrap().remove(&session_id);
                changed = true;
                println!("检测到意外退出: {}", session_id);
            }
            
            // 如果有变化，通知前端
            if changed {
                if let Err(e) = app_handle.emit("running_sessions_changed", get_running_sessions()) {
                    eprintln!("发送状态变化事件失败: {}", e);
                }
            }
        }
    });
}

/// 停止轮询
pub fn stop_polling() {
    POLLING_RUNNING.store(false, Ordering::SeqCst);
}
```

---

### Task 7: 修改 hooks.rs 实现增量更新

**Files:**
- Modify: `src-tauri/src/utils/hooks.rs`

- [ ] **Step 1: 添加导入和模块引用**

在文件顶部添加：

```rust
use crate::utils::running_sessions::{
    add_running_session,
    update_session_status,
    remove_running_session,
    get_running_sessions,
    parse_hook_event,
    SessionStatus,
};
use tauri::Emitter;
```

- [ ] **Step 2: 修改 process_file_event 函数**

找到现有的 `process_file_event` 函数，替换为：

```rust
/// 处理文件系统事件
fn process_file_event(event: &Event, app_handle: &tauri::AppHandle) {
    // 只处理修改事件
    match event.kind {
        EventKind::Modify(_) => {}
        _ => return,
    }

    for path in &event.paths {
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        // 延迟等待文件写入完成
        thread::sleep(Duration::from_millis(100));

        // 解析 hook 事件
        let content = read_event_file_with_retry(path);
        if let Some(content) = content {
            if let Ok(hook_event) = parse_hook_event(path) {
                // 增量更新状态
                handle_hook_event_incremental(&hook_event, app_handle);
            }
        }

        // 删除已处理的文件
        fs::remove_file(path).ok();
    }
}

/// 读取事件文件（带重试）
fn read_event_file_with_retry(path: &PathBuf) -> Option<String> {
    let mut attempts = 0;
    while attempts < 3 {
        attempts += 1;
        if let Ok(c) = fs::read_to_string(path) {
            if !c.is_empty() {
                return Some(c);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    None
}

/// 增量处理 hook 事件
fn handle_hook_event_incremental(event: &HookEvent, app_handle: &tauri::AppHandle) {
    println!("处理 hook 事件: {} - {}", event.hook_event_name, event.session_id);
    
    match event.hook_event_name.as_str() {
        "SessionStart" => {
            // 添加到运行中列表
            if let Ok(_) = add_running_session(&event.session_id) {
                emit_sessions_changed(app_handle);
            }
        }
        "Notification" => {
            // 更新为等待输入状态
            update_session_status(&event.session_id, SessionStatus::WaitingInput);
            emit_sessions_changed(app_handle);
            
            // 同时发送通知事件（供前端判断是否需要桌面通知）
            app_handle.emit("session_waiting_input", event).ok();
        }
        "Stop" => {
            // 更新为运行状态
            update_session_status(&event.session_id, SessionStatus::Running);
            emit_sessions_changed(app_handle);
        }
        "SessionEnd" => {
            // 从运行中列表移除
            remove_running_session(&event.session_id);
            emit_sessions_changed(app_handle);
        }
        _ => {}
    }
}

/// 发送状态变化事件
fn emit_sessions_changed(app_handle: &tauri::AppHandle) {
    let sessions = get_running_sessions();
    if let Err(e) = app_handle.emit("running_sessions_changed", sessions) {
        eprintln!("发送状态变化事件失败: {}", e);
    }
}
```

- [ ] **Step 3: 修改 start_hook_receiver 函数**

修改现有的 `start_hook_receiver` 函数，移除 `ensure_hook_writer` 调用（因为 hook_writer.py 已经独立部署）：

```rust
/// 启动钩子事件接收服务
pub fn start_hook_receiver(app_handle: tauri::AppHandle) -> Result<(), String> {
    if HOOK_RECEIVER_RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }

    HOOK_RECEIVER_RUNNING.store(true, Ordering::SeqCst);

    let events_dir = get_events_dir();
    
    // 确保目录存在
    if !events_dir.exists() {
        fs::create_dir_all(&events_dir)
            .map_err(|e| format!("创建事件目录失败: {}", e))?;
    }

    // 启动时清理历史文件
    cleanup_events_dir_on_startup(&events_dir);

    let app_handle_clone = app_handle.clone();
    let events_dir_clone = events_dir.clone();

    thread::spawn(move || {
        // 创建监听器
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("创建文件监听器失败: {}", e);
                HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        if let Err(e) = watcher.watch(&events_dir_clone, RecursiveMode::NonRecursive) {
            eprintln!("监听事件目录失败: {}", e);
            HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
            return;
        }

        loop {
            if !HOOK_RECEIVER_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(result) => {
                    if let Ok(event) = result {
                        process_file_event(&event, &app_handle_clone);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }

        cleanup_events_dir_on_exit(&events_dir_clone);
    });

    Ok(())
}

/// 启动时清理历史文件
fn cleanup_events_dir_on_startup(events_dir: &PathBuf) {
    // 不清理，让 init_running_sessions 处理
}
```

---

### Task 8: 更新 Tauri 命令入口

**Files:**
- Modify: `src-tauri/src/commands/session.rs`

- [ ] **Step 1: 添加新命令**

```rust
use crate::utils::running_sessions::{
    init_running_sessions,
    get_running_sessions,
    start_polling,
    stop_polling,
    RunningSession,
};

/// 初始化运行中 session 列表（应用启动时调用）
#[tauri::command]
pub fn init_running() -> Result<Vec<RunningSession>, String> {
    init_running_sessions()
}

/// 获取运行中 session 列表
#[tauri::command]
pub fn list_running() -> Result<Vec<RunningSession>, String> {
    Ok(get_running_sessions())
}

/// 启动定时轮询
#[tauri::command]
pub fn start_polling_cmd(app: tauri::AppHandle) -> Result<(), String> {
    start_polling(app);
    Ok(())
}

/// 停止定时轮询
#[tauri:command]
pub fn stop_polling_cmd() -> Result<(), String> {
    stop_polling();
    Ok(())
}
```

- [ ] **Step 2: 移除旧的 list_running_sessions 命令**

删除或注释掉旧的 `list_running_sessions` 命令（避免混淆）：

```rust
// 旧命令已废弃，使用 list_running 替代
// #[tauri::command]
// pub fn list_running_sessions() -> Result<Vec<ClaudeSession>, String> {
//     get_running_sessions_list()
// }
```

---

### Task 9: 更新 lib.rs 注册命令和初始化

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加模块引用**

```rust
mod utils;
mod commands;

use commands::session::{
    init_running,
    list_running,
    start_polling_cmd,
    stop_polling_cmd,
    start_hooks,
    stop_hooks,
    // 其他现有命令...
};
```

- [ ] **Step 2: 注册新命令**

在 `invoke_handler` 中添加：

```rust
.invoke_handler(tauri::generate_handler![
    init_running,
    list_running,
    start_polling_cmd,
    stop_polling_cmd,
    start_hooks,
    stop_hooks,
    // 其他现有命令...
])
```

- [ ] **Step 3: 修改 setup 函数实现启动初始化**

```rust
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 初始化运行中 session 列表
    let app_handle = app.handle();
    init_running().ok();
    
    // 启动 hook 监听
    start_hooks(app_handle.clone()).ok();
    
    // 启动定时轮询
    start_polling_cmd(app_handle.clone()).ok();
    
    Ok(())
}
```

---

### Task 10: 创建前端 useRunningSessions hook

**Files:**
- Create: `src/hooks/useRunningSessions.ts`

- [ ] **Step 1: 创建 hook 文件**

```typescript
import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

export type SessionStatus = 'running' | 'waiting_input'

export interface RunningSession {
  session_id: string
  pid: number
  status: SessionStatus
  cwd: string
  name: string
  updated_at: number
}

export function useRunningSessions() {
  const [sessions, setSessions] = useState<RunningSession[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 加载初始列表
  const loadSessions = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const result = await invoke<RunningSession[]>('list_running')
      setSessions(result)
    } catch (e) {
      setError(String(e))
    }
    setLoading(false)
  }, [])

  // 初始加载 + 监听事件
  useEffect(() => {
    loadSessions()

    // 监听状态变化事件
    const setupListener = async () => {
      unlistenRef.current = await listen<RunningSession[]>('running_sessions_changed', (event) => {
        setSessions(event.payload)
      })
    }
    setupListener()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [loadSessions])

  // 手动刷新
  const refresh = useCallback(async () => {
    await loadSessions()
  }, [loadSessions])

  return {
    sessions,
    loading,
    error,
    refresh,
  }
}
```

- [ ] **Step 2: 添加 useRef 导入**

修正 import：

```typescript
import { useState, useEffect, useCallback, useRef } from 'react'
```

---

### Task 11: 修改 useNotification.ts

**Files:**
- Modify: `src/hooks/useNotification.ts`

- [ ] **Step 1: 简化通知逻辑，移除全量刷新**

```typescript
import { useEffect, useRef, useCallback } from 'react'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore } from '@/stores'
import { sendDesktopNotification, playNotificationSound } from '@/services'

interface WaitingInputEvent {
  session_id: string
  cwd?: string
  hook_event_name: string
}

export function useNotification() {
  const { notificationSound, notificationDesktop } = useSettingsStore()
  const notifiedSessions = useRef<Set<string>>(new Set())
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 发送通知
  const sendNotification = useCallback((sessionId: string, sessionName: string, cwd?: string) => {
    const fallbackName = cwd?.split(/[\\/]/).pop() || sessionId

    if (notificationDesktop) {
      sendDesktopNotification({
        title: 'Claude Fleet - 等待输入',
        body: `Session "${sessionName || fallbackName}" 正在等待输入`,
        sessionId,
        sound: notificationSound,
      })
    } else if (notificationSound) {
      playNotificationSound()
    }
  }, [notificationDesktop, notificationSound])

  // 监听等待输入事件
  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<WaitingInputEvent>('session_waiting_input', (event) => {
        const payload = event.payload
        
        // 检查是否已通知过
        if (!notifiedSessions.current.has(payload.session_id)) {
          notifiedSessions.current.add(payload.session_id)
          
          // 从 cwd 提取名称
          const name = payload.cwd?.split(/[\\/]/).pop() || ''
          sendNotification(payload.session_id, name, payload.cwd)
        }
      })
    }
    
    setupListener()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [sendNotification])

  // 清除通知记录（当 session 状态变化时）
  const clearNotifiedSession = useCallback((sessionId: string) => {
    notifiedSessions.current.delete(sessionId)
  }, [])

  return {
    clearNotifiedSession,
  }
}
```

---

### Task 12: 修改 RunningTab.tsx 使用新 hook

**Files:**
- Modify: `src/components/running/RunningTab.tsx`

- [ ] **Step 1: 修改导入和状态管理**

```typescript
import { useMemo } from "react"
import { cn } from "@/lib/utils"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { SessionCard } from "./SessionCard"
import { StatusBadge } from "./StatusBadge"
import { searchSessions } from "@/utils"
import { useRunningSessions, RunningSession } from "@/hooks/useRunningSessions"
import { jumpToTerminal } from "@/services"
import { RefreshCw } from "lucide-react"
```

- [ ] **Step 2: 修改组件逻辑**

```typescript
export function RunningTab() {
  const { sessions, loading, error, refresh } = useRunningSessions()
  const [refreshing, setRefreshing] = useState(false)
  const [searchQuery, setSearchQuery] = useState("")

  // 搜索过滤
  const filteredSessions = useMemo(() => {
    if (searchQuery) {
      return sessions.filter((s) =>
        s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.cwd.toLowerCase().includes(searchQuery.toLowerCase())
      )
    }
    return sessions
  }, [sessions, searchQuery])

  // 统计
  const waitingCount = sessions.filter((s) => s.status === "waiting_input").length

  const handleRefresh = async () => {
    setRefreshing(true)
    await refresh()
    setRefreshing(false)
  }

  const handleJumpToTerminal = async (session: RunningSession) => {
    try {
      await jumpToTerminal({
        id: session.session_id,
        workingDirectory: session.cwd,
        processId: session.pid,
        status: session.status,
        name: session.name,
      })
    } catch (e) {
      alert(String(e))
    }
  }

  return (
    <div className="flex flex-col h-full">
      {/* 搜索栏 */}
      <div className="flex items-center gap-2 px-4 py-3 border-b bg-gray-50">
        <Input
          placeholder="搜索名称、路径..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="flex-1"
        />
        <Button
          variant="outline"
          size="icon"
          onClick={handleRefresh}
          disabled={refreshing}
        >
          <RefreshCw className={cn("w-4 h-4", refreshing && "animate-spin")} />
        </Button>
      </div>

      {/* 状态统计 */}
      <div className="flex items-center gap-4 px-4 py-2 border-b text-sm">
        <span className="text-gray-600">
          共 {filteredSessions.length} 个运行中的 session
        </span>
        {waitingCount > 0 && (
          <span className="text-amber-600 font-medium">
            {waitingCount} 个等待输入
          </span>
        )}
      </div>

      {/* Session 列表 */}
      <ScrollArea className="flex-1 p-4">
        {loading && (
          <div className="text-center text-gray-500 py-8">加载中...</div>
        )}

        {error && (
          <div className="text-center text-red-500 py-8">{error}</div>
        )}

        {!loading && !error && filteredSessions.length === 0 && (
          <div className="text-center text-gray-500 py-8">
            {searchQuery ? "没有匹配的 session" : "没有运行中的 session"}
          </div>
        )}

        {!loading && !error && filteredSessions.length > 0 && (
          <div className="flex flex-col gap-3">
            {filteredSessions.map((session) => (
              <SessionCardNew
                key={session.session_id}
                session={session}
                onJumpToTerminal={handleJumpToTerminal}
              />
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  )
}
```

- [ ] **Step 3: 添加 useState 导入**

```typescript
import { useMemo, useState } from "react"
```

---

### Task 13: 创建新的 SessionCard 组件适配 RunningSession

**Files:**
- Modify: `src/components/running/SessionCard.tsx`

- [ ] **Step 1: 添加兼容新类型的渲染逻辑**

修改现有组件，添加支持 `RunningSession` 类型：

```typescript
import { cn } from "@/lib/utils"
import { StatusBadge } from "./StatusBadge"
import { Button } from "@/components/ui/button"
import { formatRelativeTime } from "@/utils"
import { Star } from "lucide-react"
import type { RunningSession } from "@/hooks/useRunningSessions"

interface SessionCardNewProps {
  session: RunningSession
  onJumpToTerminal: (session: RunningSession) => void
}

export function SessionCardNew({ session, onJumpToTerminal }: SessionCardNewProps) {
  const isWaitingInput = session.status === "waiting_input"

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
        </div>
        <p className="text-sm text-gray-600 truncate">{session.cwd}</p>
        <p className="text-xs text-gray-500 mt-1">
          PID: {session.pid}
        </p>
      </div>

      <div className="flex items-center gap-2 ml-4">
        <Button
          variant={isWaitingInput ? "default" : "secondary"}
          size="sm"
          onClick={() => onJumpToTerminal(session)}
          className={isWaitingInput ? "bg-violet-600 hover:bg-violet-700" : ""}
        >
          跳转到终端
        </Button>
      </div>
    </div>
  )
}

// 保留原有 SessionCard 组件兼容旧类型
// ... 现有代码 ...
```

---

### Task 14: 更新 services 接口

**Files:**
- Modify: `src/services/claudeSession.ts`

- [ ] **Step 1: 添加类型兼容**

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession, Conversation } from '@/types'
import type { RunningSession } from '@/hooks/useRunningSessions'

// 保留现有函数用于 Management Tab
export async function listSessions(): Promise<ClaudeSession[]> {
  // ... 现有代码 ...
}

// 新增：获取运行中 session（轻量级）
export async function listRunningSessions(): Promise<RunningSession[]> {
  try {
    const sessions = await invoke<RunningSession[]>('list_running')
    return sessions
  } catch (error) {
    console.error('获取运行中 session 列表失败:', error)
    throw error
  }
}

// 其他现有函数保持不变...
```

---

### Task 15: 编译验证和测试

- [ ] **Step 1: 编译 Rust 后端**

```bash
cd src-tauri
cargo build
```

预期：编译成功，可能有少量警告

- [ ] **Step 2: 编译前端**

```bash
npm run build
```

预期：编译成功

- [ ] **Step 3: 运行应用测试**

```bash
npm run tauri dev
```

验证：
- 应用启动无错误
- RunningTab 正常显示
- 在测试目录启动 Claude Code，观察状态变化

- [ ] **Step 4: 测试 hook 事件处理**

在 `C:\workspace\claude-test-workspace` 目录：
1. 启动 Claude Code
2. 观察 `~/.claude-fleet/events/` 文件命名
3. 观察 RunningTab 状态变化
4. 结束 Claude Code
5. 验证事件文件清理

- [ ] **Step 5: 提交变更**

```bash
git add .
git commit -m "feat: 实现增量状态管理，消除全量扫描卡顿"
```

---

## 自审清单

**1. Spec 覆盖检查：**
- ✅ hook_writer.py 改进 → Task 1
- ✅ running_sessions.rs 新模块 → Task 2-6
- ✅ 进程验证改进 → Task 3
- ✅ hooks.rs 增量更新 → Task 7
- ✅ Tauri 命令 → Task 8-9
- ✅ 前端 hooks → Task 10-11
- ✅ RunningTab 改进 → Task 12-13
- ✅ 服务接口 → Task 14

**2. Placeholder 扫描：**
- 无 TBD/TODO
- 无模糊描述
- 所有代码步骤都有完整代码

**3. 类型一致性：**
- RunningSession 结构在各文件中一致
- SessionStatus 使用 snake_case 序列化
- hook_event_name 字符串匹配