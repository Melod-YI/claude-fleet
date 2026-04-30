# 增量状态管理设计

## 问题背景

当前"运行中"页面存在严重的卡顿问题，主要原因：

1. **useNotification.ts** 收到任何 hook 事件都调用 `refresh()` → `get_all_sessions()` → 全量扫描所有历史 session 文件
2. 每次扫描都要遍历 `~/.claude/projects/**/*.jsonl` + 解析全部内容 + 对每个 session 执行 `tasklist` 命令
3. hook 文件使用 timestamp 命名，容易堆积成百上千个文件

## 设计目标

- 应用启动时只读取积压的 hook 事件文件，不扫描全量 session
- 运行时 hook 事件触发增量更新，不触发全量扫描
- 定时轮询检测意外退出（30s 间隔）
- 只有用户打开"历史 session 管理"页面时才触发全量扫描

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    增量状态管理架构                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Claude Code CLI                                            │
│       │                                                      │
│       │ hook 事件                                            │
│       ▼                                                      │
│  hook_writer.py ──► ~/.claude-fleet/events/                 │
│                         │                                    │
│                         │ <session-id>_<event-type>.json     │
│                         │ (最多 3 个文件/session)             │
│                         ▼                                    │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Tauri 后端                   ││
│  ├─────────────────────────────────────────────────────────┤│
│  │                                                         ││
│  │  running_sessions.rs (新增)                             ││
│  │  ┌─────────────────────────────────────────────────┐   ││
│  │  │ RUNNING_SESSIONS: Mutex<HashMap<SessionId,      │   ││
│  │  │   RunningSession { pid, status, cwd, updatedAt }│   ││
│  │  └─────────────────────────────────────────────────┘   ││
│  │                                                         ││
│  │  启动时：                                                ││
│  │  1. 读取 events/*.json，按 session_id 分组              ││
│  │  2. 有 SessionEnd → 跳过                                ││
│  │  3. 只有 SessionStart → 查 PID 验证                     ││
│  │  4. 清理已处理事件文件                                   ││
│  │                                                         ││
│  │  运行时：                                                ││
│  │  1. hooks.rs 监听 → 增量更新 RUNNING_SESSIONS           ││
│  │  2. 定时轮询 (30s) → 检测意外退出                        ││
│  │  3. emit("running_sessions_changed") → 前端            ││
│  │                                                         ││
│  └─────────────────────────────────────────────────────────┘│
│                         │                                    │
│                         ▼                                    │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              前端                          ││
│  ├─────────────────────────────────────────────────────────┤│
│  │                                                         ││
│  │  RunningTab: invoke("list_running") 获取列表            ││
│  │  useNotification: listen("running_sessions_changed")    ││
│  │                                                         ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 组件设计

### 1. hook_writer.py 改进

**文件命名策略：** `<session-id>_<event-type>.json`

```
~/.claude-fleet/events/
├── 189204dc-7214_SessionStart.json
├── 189204dc-7214_Notification.json
├── 189204dc-7214_Stop.json
├── 189204dc-7214_SessionEnd.json  ← 写入时清理该 session 所有文件
└── abc123-4567_SessionStart.json
```

**改进逻辑：**

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
    # 仍然写入 SessionEnd 文件，供启动时判断
    file_path = os.path.join(events_dir, f"{session_id}_{event_type}.json")
else:
    # 其他事件：覆盖同类型旧文件
    file_path = os.path.join(events_dir, f"{session_id}_{event_type}.json")

with open(file_path, "w", encoding="utf-8") as f:
    json.dump(hook_input, f, indent=2, ensure_ascii=False)
```

**优势：**
- 每个 session 最多 4 个文件
- SessionEnd 时主动清理，避免堆积
- 启动时扫描压力极小

### 2. 后端 running_sessions.rs (新增)

**状态结构：**

```rust
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Running,
    WaitingInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningSession {
    pub session_id: String,
    pub pid: u32,
    pub status: SessionStatus,
    pub cwd: String,
    pub name: String,
    pub updated_at: u64,
}

pub static RUNNING_SESSIONS: Lazy<Mutex<HashMap<String, RunningSession>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));
```

**Tauri 命令：**

```rust
#[tauri::command]
pub fn init_running_sessions(app: tauri::AppHandle) -> Result<(), String> {
    // 1. 读取 events/*.json
    // 2. 按 session_id 分组判断状态
    // 3. 验证 PID + 进程名
    // 4. 清理已处理文件
}

#[tauri::command]
pub fn list_running_sessions() -> Result<Vec<RunningSession>, String> {
    Ok(RUNNING_SESSIONS.lock().unwrap().values().cloned().collect())
}
```

**初始化流程：**

```
读取 ~/.claude-fleet/events/*.json
    │
    ▼
按 session_id 分组
    │
    ├─ 有 SessionEnd.json → 跳过（已结束）
    │
    └─ 只有 SessionStart → 可能运行中
        │
        ▼
    查 ~/.claude/sessions/<session-id>.json 获取 PID
        │
        ▼
    is_process_running(pid) && 进程名 == "claude"
        │
        ├─ 存在 → 加入 RUNNING_SESSIONS
        │
        └─ 不存在 → 跳过（意外退出）
    │
    ▼
清理已处理的 events 文件
```

### 3. hooks.rs 改进

**当前：** 监听文件 → emit("hook_event") → 前端处理（触发全量扫描）

**改进：** 监听文件 → 直接更新 RUNNING_SESSIONS → emit("running_sessions_changed")

```rust
fn process_file_event(event: &Event, app_handle: &tauri::AppHandle) {
    let hook_event = parse_hook_file(path);
    
    match hook_event.hook_event_name.as_str() {
        "SessionStart" => {
            // 读取 sessions/*.json 获取 PID
            // 验证进程 → 加入 RUNNING_SESSIONS
            add_running_session(session_id, pid, cwd);
        }
        "Notification" => {
            // 更新 status = WaitingInput
            update_session_status(session_id, SessionStatus::WaitingInput);
        }
        "Stop" => {
            // 更新 status = Running
            update_session_status(session_id, SessionStatus::Running);
        }
        "SessionEnd" => {
            // 从 RUNNING_SESSIONS 移除
            remove_running_session(session_id);
        }
    }
    
    // 删除已处理的文件
    fs::remove_file(path);
    
    // 通知前端
    app_handle.emit("running_sessions_changed", list_running_sessions());
}
```

### 4. 定时轮询（检测意外退出）

```rust
pub fn start_polling(app_handle: tauri::AppHandle) {
    thread::spawn(|| {
        loop {
            sleep(Duration::from_secs(30));
            
            let mut changed = false;
            for (session_id, session) in RUNNING_SESSIONS.lock().unwrap().iter_mut() {
                if !is_process_running(session.pid) {
                    RUNNING_SESSIONS.lock().unwrap().remove(session_id);
                    changed = true;
                }
            }
            
            if changed {
                app_handle.emit("running_sessions_changed", ...);
            }
        }
    });
}
```

### 5. 进程检查改进

**当前问题：** 只检查 PID 是否存在，不验证进程名

**改进：** 同时验证进程名是否为 "claude"

```rust
fn is_claude_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 检查 PID 存在且进程名包含 "claude"
            stdout.contains(&pid.to_string()) && stdout.contains("claude")
        } else {
            false
        }
    }
}
```

### 6. 前端改进

**useNotification.ts：**

```typescript
// 当前：收到 hook_event → refresh() → 全量扫描
// 改进：收到 running_sessions_changed → 直接更新状态

listen("running_sessions_changed", (event) => {
    const sessions = event.payload as RunningSession[];
    
    // 直接更新状态（轻量级）
    setRunningSessions(sessions);
    
    // 检查是否有 waiting_input 需要通知
    const waitingSession = sessions.find(s => s.status === 'WaitingInput');
    if (waitingSession && !notifiedSessions.has(waitingSession.session_id)) {
        sendNotification(waitingSession);
        notifiedSessions.add(waitingSession.session_id);
    }
});
```

**RunningTab.tsx：**

```typescript
// 使用新的 hook 获取状态
const { runningSessions, loading } = useRunningSessions();

// 不再需要手动 loadSessions()
// 状态由后端推送，前端只需监听
```

## 性能对比

| 操作 | 当前耗时 | 设计后耗时 |
|------|---------|-----------|
| 应用启动 | 扫描所有 sessions/*.json + 所有 jsonl + N 次 tasklist | 只读取几个 events 文件 + M 次 tasklist（M << N） |
| hook 事件 | refresh() → 全量扫描（秒级） | 增量更新 RUNNING_SESSIONS（毫秒级） |
| 检测意外退出 | 每次 hook 都检查所有 | 定时轮询运行中列表（30s） |
| 打开 Management Tab | loadSessions() → 全量扫描 | 保持不变（用户主动触发） |

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `docs/hooks/hook_writer.py` | 修改 | 改文件命名策略 + SessionEnd 清理 |
| `src-tauri/src/utils/running_sessions.rs` | 新增 | 状态管理 + 命令 |
| `src-tauri/src/utils/hooks.rs` | 修改 | 增量更新 RUNNING_SESSIONS |
| `src-tauri/src/utils/claude_data.rs` | 修改 | 添加 `is_claude_process_running` |
| `src-tauri/src/lib.rs` | 修改 | 注册新命令 + 启动时初始化 |
| `src-tauri/src/commands/session.rs` | 修改 | 新增命令入口 |
| `src/hooks/useNotification.ts` | 修改 | 监听 running_sessions_changed |
| `src/hooks/useRunningSessions.ts` | 新增 | 运行中状态管理 hook |
| `src/components/running/RunningTab.tsx` | 修改 | 使用新 hook |
| `src/services/claudeSession.ts` | 修改 | 更新服务接口 |

## 测试验证

测试目录：`C:\workspace\claude-test-workspace`
Hook 配置：`C:\workspace\claude-test-workspace\.claude\settings.json`

验证步骤：
1. 启动 claude-fleet 应用，观察启动耗时
2. 在测试目录启动 Claude Code，观察 hook 事件处理
3. 观察 RunningTab 是否正确显示状态变化
4. 结束 Claude Code，观察 SessionEnd 清理效果
5. 验证意外退出检测（强制关闭 Claude Code 进程）