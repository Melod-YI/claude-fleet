use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;
use tauri::Emitter;
use crate::utils::claude_data::is_claude_process_running;

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

/// 轮询运行状态
static POLLING_RUNNING: AtomicBool = AtomicBool::new(false);

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

/// 从路径提取最后一段作为名称
fn get_path_name(path: &str) -> String {
    path.split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

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

/// 启动定时轮询（检测意外退出）
pub fn start_polling(app_handle: tauri::AppHandle) {
    if POLLING_RUNNING.load(Ordering::SeqCst) {
        return;
    }

    POLLING_RUNNING.store(true, Ordering::SeqCst);

    let app_handle_clone = app_handle.clone();

    thread::spawn(move || {
        loop {
            if !POLLING_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            thread::sleep(Duration::from_secs(30));

            // 检查所有运行中 session 的进程状态
            let sessions_to_remove: Vec<String> = RUNNING_SESSIONS
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, session)| !is_claude_process_running(session.pid))
                .map(|(id, _)| id.clone())
                .collect();

            for session_id in sessions_to_remove {
                RUNNING_SESSIONS.lock().unwrap().remove(&session_id);
                println!("检测到意外退出: {}", session_id);
            }

            // 通知前端
            if let Err(e) = app_handle_clone.emit("running_sessions_changed", get_running_sessions()) {
                eprintln!("发送状态变化事件失败: {}", e);
            }
        }
    });
}

/// 停止轮询
pub fn stop_polling() {
    POLLING_RUNNING.store(false, Ordering::SeqCst);
}