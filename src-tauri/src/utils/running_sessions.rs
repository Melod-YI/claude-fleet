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
use tracing::{info, debug, warn, error};
use std::time::Instant;
use crate::utils::claude_data::is_claude_process_running;
use crate::utils::window_manager::get_window_title_by_pid_chain;

/// Session 运行状态（对应 Claude JSON 文件中的三种状态）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Busy,        // agent 与 LLM 在 loop 中
    Idle,        // 等待用户输入
    Waiting,     // 等待用户输入
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

/// 全局运行中 Session 状态（PID 作为 key，因为 sessionId 会因 resume 变化）
pub static RUNNING_SESSIONS: Lazy<Mutex<HashMap<u32, RunningSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 轮询运行状态
static POLLING_RUNNING: AtomicBool = AtomicBool::new(false);

/// Hook 事件结构（用于前端通知事件兼容）
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

/// Session 文件内容结构（从 ~/.claude/sessions/<pid>.json）
#[derive(Debug, Clone, Deserialize)]
pub struct SessionFileContent {
    pub pid: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub cwd: String,
    #[serde(rename = "startedAt")]
    pub started_at: u64,
    #[serde(default)]
    pub status: String,
    #[serde(rename = "waitingFor", default)]
    pub waiting_for: Option<String>,
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
}

/// 获取 sessions 目录路径
pub fn get_sessions_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude")
        .join("sessions")
}

/// 从路径提取最后一段作为名称
pub fn get_path_name(path: &str) -> String {
    path.split(|c| c == '\\' || c == '/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

/// 解析 session 名称（优先级：自定义名称 > 窗口名 > 文件夹名）
pub fn resolve_session_name(content: &SessionFileContent) -> String {
    info!("[resolve_session_name] 开始解析名称: pid={}", content.pid);

    // 1. 优先使用用户自定义名称
    if let Some(custom_name) = &content.name {
        if !custom_name.is_empty() {
            info!("[resolve_session_name] 使用自定义名称: {}", custom_name);
            return custom_name.clone();
        }
    }

    // 2. 尝试获取窗口标题
    if let Some(window_title) = get_window_title_by_pid_chain(content.pid) {
        // 判断是否为默认标题（忽略大小写，以 "claude code" 或 "claude-code" 结尾）
        let title_lower = window_title.trim().to_lowercase();
        let is_default_title = title_lower.ends_with("claude code") || title_lower.ends_with("claude-code");

        if !is_default_title && !window_title.is_empty() {
            info!("[resolve_session_name] 使用窗口标题: {}", window_title);
            return window_title;
        }
        debug!("[resolve_session_name] 窗口标题为默认值 \"{}\"，使用文件夹名", window_title);
    }

    // 3. 使用文件夹名称
    let folder_name = get_path_name(&content.cwd);
    info!("[resolve_session_name] 使用文件夹名: {}", folder_name);
    folder_name
}

/// 从文件名解析 PID："33804.json" -> 33804
pub fn parse_pid_from_filename(filename: &str) -> Result<u32, String> {
    filename
        .strip_suffix(".json")
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| format!("无法从文件名解析 PID: {}", filename))
}

/// 从文件内容添加 session 到运行中列表
pub fn add_running_session_from_file(content: &SessionFileContent) -> Result<(), String> {
    info!("[add_running_session_from_file] 开始添加: pid={}, sessionId={}", content.pid, content.session_id);
    let start = Instant::now();

    // 验证进程是否为 claude
    debug!("[add_running_session_from_file] 检查进程 PID={} 是否为 claude", content.pid);
    if !is_claude_process_running(content.pid) {
        warn!("[add_running_session_from_file] PID {} 不是 claude 进程或进程已退出", content.pid);
        return Err(format!("PID {} 不是 claude 进程", content.pid));
    }
    info!("[add_running_session_from_file] PID {} 确认是 claude 进程", content.pid);

    // 提取名称（优先级：自定义名称 > 窗口名 > 文件夹名）
    let name = resolve_session_name(content);
    debug!("[add_running_session_from_file] 最终名称: {}", name);

    // 状态映射：busy -> Busy, idle -> Idle, waiting -> Waiting
    let status = match content.status.as_str() {
        "busy" => {
            debug!("[add_running_session_from_file] status=busy -> Busy");
            SessionStatus::Busy
        }
        "idle" => {
            debug!("[add_running_session_from_file] status=idle -> Idle");
            SessionStatus::Idle
        }
        "waiting" => {
            debug!("[add_running_session_from_file] status=waiting -> Waiting");
            SessionStatus::Waiting
        }
        _ => {
            warn!("[add_running_session_from_file] status={} 未知，默认为 Busy", content.status);
            SessionStatus::Busy
        }
    };

    let session = RunningSession {
        session_id: content.session_id.clone(),
        pid: content.pid,
        status,
        cwd: content.cwd.clone(),
        name,
        updated_at: content.updated_at.unwrap_or(content.started_at),
    };

    info!("[add_running_session_from_file] 创建 RunningSession: id={}, pid={}, status={}, cwd={}",
          session.session_id, session.pid,
          match status {
              SessionStatus::Busy => "busy",
              SessionStatus::Idle => "idle",
              SessionStatus::Waiting => "waiting",
          },
          session.cwd);

    // 添加到全局状态（PID 作为 key）
    RUNNING_SESSIONS.lock().unwrap().insert(content.pid, session);

    let elapsed = start.elapsed();
    info!("[add_running_session_from_file] 完成，耗时: {}ms", elapsed.as_millis());
    Ok(())
}

/// 从文件内容更新 session 状态（PID 作为查找 key）
pub fn update_session_status_from_file(content: &SessionFileContent) {
    info!("[update_session_status_from_file] 开始更新: pid={}, sessionId={}, status={}",
          content.pid, content.session_id, content.status);

    let mut sessions = RUNNING_SESSIONS.lock().unwrap();

    // 用 PID 查找（sessionId 会因 resume 变化）
    if let Some(session) = sessions.get_mut(&content.pid) {
        // 检查 sessionId 是否变化（resume 情况）
        if session.session_id != content.session_id {
            info!("[update_session_status_from_file] sessionId 变化: {} -> {}",
                  session.session_id, content.session_id);
            session.session_id = content.session_id.clone();
        }

        let new_status = match content.status.as_str() {
            "busy" => SessionStatus::Busy,
            "idle" => SessionStatus::Idle,
            "waiting" => SessionStatus::Waiting,
            _ => SessionStatus::Busy,
        };

        let old_status = session.status;
        session.status = new_status;
        session.updated_at = content.updated_at.unwrap_or(session.updated_at);

        info!("[update_session_status_from_file] 完成: {} -> {}",
              match old_status {
                  SessionStatus::Busy => "busy",
                  SessionStatus::Idle => "idle",
                  SessionStatus::Waiting => "waiting",
              },
              match new_status {
                  SessionStatus::Busy => "busy",
                  SessionStatus::Idle => "idle",
                  SessionStatus::Waiting => "waiting",
              });
    } else {
        warn!("[update_session_status_from_file] 未找到 PID={} 的 session", content.pid);
    }
}

/// 根据 PID 移除 session（PID 作为 key，直接移除）
pub fn remove_running_session_by_pid(pid: u32) {
    info!("[remove_running_session_by_pid] 开始移除: pid={}", pid);

    let mut sessions = RUNNING_SESSIONS.lock().unwrap();

    if let Some(session) = sessions.remove(&pid) {
        info!("[remove_running_session_by_pid] 完成: pid={}, sessionId={} 已移除，剩余 {} 个 session",
              pid, session.session_id, sessions.len());
    } else {
        warn!("[remove_running_session_by_pid] 未找到 pid={} 的 session", pid);
    }
}

/// 获取所有运行中 session
pub fn get_running_sessions() -> Vec<RunningSession> {
    debug!("[get_running_sessions] 开始获取");

    let sessions: Vec<RunningSession> = RUNNING_SESSIONS.lock().unwrap().values().cloned().collect();

    let busy_count = sessions.iter().filter(|s| s.status == SessionStatus::Busy).count();
    let idle_count = sessions.iter().filter(|s| s.status == SessionStatus::Idle).count();
    let waiting_count = sessions.iter().filter(|s| s.status == SessionStatus::Waiting).count();
    let waiting_input_total = idle_count + waiting_count;

    debug!("[get_running_sessions] 完成: 总数={}, busy={}, idle={}, waiting={}, 等待输入总数={}",
           sessions.len(), busy_count, idle_count, waiting_count, waiting_input_total);
    sessions
}

/// 应用启动时初始化运行中 session 列表（扫描 sessions 目录）
pub fn init_running_sessions() -> Result<Vec<RunningSession>, String> {
    info!("[init_running_sessions] 开始初始化运行中 session 列表");
    let start = Instant::now();

    let sessions_dir = get_sessions_dir();
    debug!("[init_running_sessions] sessions 目录: {}", sessions_dir.display());

    if !sessions_dir.exists() {
        info!("[init_running_sessions] sessions 目录不存在，返回空列表");
        return Ok(Vec::new());
    }

    // 清空现有状态
    debug!("[init_running_sessions] 清空 RUNNING_SESSIONS");
    RUNNING_SESSIONS.lock().unwrap().clear();

    // 扫描 sessions 目录下的所有 JSON 文件
    let entries = fs::read_dir(&sessions_dir)
        .map_err(|e| {
            error!("[init_running_sessions] 读取目录失败: {}", e);
            format!("读取 sessions 目录失败: {}", e)
        })?;

    let mut total_files = 0;
    let mut json_files = 0;
    let mut added_count = 0;
    let mut parse_failed = 0;
    let mut process_dead = 0;

    for entry in entries {
        let file_path = entry.map_err(|e| {
            warn!("[init_running_sessions] 读取条目失败: {}", e);
            format!("读取条目失败: {}", e)
        })?.path();

        total_files += 1;

        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        json_files += 1;
        debug!("[init_running_sessions] 检查文件 #{}: {}", json_files, file_path.display());

        // 解析文件内容
        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(session_content) = serde_json::from_str::<SessionFileContent>(&content) {
                debug!("[init_running_sessions] 解析成功: pid={}, sessionId={}, status={}",
                       session_content.pid, session_content.session_id, session_content.status);

                // 验证进程是否存活
                if !is_claude_process_running(session_content.pid) {
                    process_dead += 1;
                    debug!("[init_running_sessions] PID {} 进程已退出，跳过", session_content.pid);
                    continue;
                }

                // 添加到运行中列表
                if add_running_session_from_file(&session_content).is_ok() {
                    added_count += 1;
                }
            } else {
                parse_failed += 1;
                warn!("[init_running_sessions] 解析失败: {}", file_path.display());
            }
        } else {
            parse_failed += 1;
            warn!("[init_running_sessions] 读取文件失败: {}", file_path.display());
        }
    }

    info!("[init_running_sessions] 统计: 总文件={}, json={}, 解析失败={}, 进程已退出={}, 成功添加={}",
          total_files, json_files, parse_failed, process_dead, added_count);

    let result = get_running_sessions();
    let elapsed = start.elapsed();
    info!("[init_running_sessions] 完成，运行中 session 数量: {}，耗时: {}ms", result.len(), elapsed.as_millis());
    Ok(result)
}

/// 启动定时轮询（检测意外退出）
pub fn start_polling(app_handle: tauri::AppHandle) {
    info!("[start_polling] 开始启动轮询服务");

    if POLLING_RUNNING.load(Ordering::SeqCst) {
        warn!("[start_polling] 轮询服务已在运行，跳过启动");
        return;
    }

    info!("[start_polling] 设置运行标志并启动线程（30秒间隔）");
    POLLING_RUNNING.store(true, Ordering::SeqCst);

    let app_handle_clone = app_handle.clone();

    thread::spawn(move || {
        info!("[polling_thread] 轮询线程启动");
        let mut poll_count = 0;

        loop {
            if !POLLING_RUNNING.load(Ordering::SeqCst) {
                info!("[polling_thread] 收到停止信号，退出线程（共轮询 {} 次）", poll_count);
                break;
            }

            thread::sleep(Duration::from_secs(30));
            poll_count += 1;

            let poll_start = Instant::now();
            info!("[polling_thread] 轮询 #{} 开始", poll_count);

            let session_count = RUNNING_SESSIONS.lock().unwrap().len();
            debug!("[polling_thread] 当前 session 数量: {}", session_count);

            if session_count == 0 {
                debug!("[polling_thread] 无 session，跳过进程检查");
                continue;
            }

            // 检查每个 session 的进程状态（PID 作为 key）
            let pids_to_remove: Vec<u32> = RUNNING_SESSIONS
                .lock()
                .unwrap()
                .iter()
                .filter(|(pid, session)| {
                    let running = is_claude_process_running(**pid);
                    if !running {
                        warn!("[polling_thread] PID {} (session {}) 已退出", pid, session.session_id);
                    }
                    !running
                })
                .map(|(pid, _)| *pid)
                .collect();

            let removed_count = pids_to_remove.len();
            if removed_count > 0 {
                info!("[polling_thread] 检测到 {} 个意外退出的 session", removed_count);
            }

            for pid in &pids_to_remove {
                info!("[polling_thread] 移除 PID: {}", pid);
                RUNNING_SESSIONS.lock().unwrap().remove(pid);
            }

            // 通知前端
            let sessions = get_running_sessions();

            if let Err(e) = app_handle_clone.emit("running_sessions_changed", sessions) {
                error!("[polling_thread] 发送事件失败: {}", e);
            } else {
                info!("[polling_thread] 事件发送成功");
            }

            let elapsed = poll_start.elapsed();
            info!("[polling_thread] 轮询 #{} 完成，耗时: {}ms", poll_count, elapsed.as_millis());
        }

        info!("[polling_thread] 轮询线程退出");
    });

    info!("[start_polling] 完成，轮询线程已启动");
}

/// 停止轮询
pub fn stop_polling() {
    info!("[stop_polling] 开始停止轮询服务");
    POLLING_RUNNING.store(false, Ordering::SeqCst);
    info!("[stop_polling] 完成，运行标志已设置为 false");
}