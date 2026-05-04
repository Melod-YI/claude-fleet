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
    info!("[parse_hook_event] 开始解析: {}", file_path.display());
    let start = Instant::now();

    debug!("[parse_hook_event] 读取文件内容");
    let content = fs::read_to_string(file_path)
        .map_err(|e| {
            error!("[parse_hook_event] 读取失败 {}: {}", file_path.display(), e);
            format!("读取事件文件失败: {}", e)
        })?;

    debug!("[parse_hook_event] 文件内容长度: {} 字节", content.len());
    debug!("[parse_hook_event] 文件内容: {}", content);

    let result = serde_json::from_str::<HookEvent>(&content)
        .map_err(|e| {
            error!("[parse_hook_event] JSON 解析失败 {}: {}", file_path.display(), e);
            format!("解析事件 JSON 失败: {}", e)
        })?;

    let elapsed = start.elapsed();
    info!("[parse_hook_event] 完成: session_id={}, event_type={}, 耗时: {}ms",
          result.session_id, result.hook_event_name, elapsed.as_millis());
    Ok(result)
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

/// 读取 session 元数据（通过 session_id 查找）
fn read_session_metadata(session_id: &str) -> Result<SessionMetadata, String> {
    info!("[read_session_metadata] 开始查找 session_id: {}", session_id);
    let start = Instant::now();

    let sessions_dir = get_sessions_dir();
    debug!("[read_session_metadata] sessions 目录: {}", sessions_dir.display());

    // sessions/*.json 文件名是 PID，需要遍历查找 sessionId 匹配的文件
    let entries = fs::read_dir(&sessions_dir)
        .map_err(|e| {
            error!("[read_session_metadata] 读取目录失败: {}", e);
            format!("读取 sessions 目录失败: {}", e)
        })?;

    let mut checked_files = 0;
    let mut parse_success = 0;
    let mut parse_fail = 0;

    for entry in entries {
        let file_path = entry.map_err(|e| {
            warn!("[read_session_metadata] 读取条目失败: {}", e);
            format!("读取条目失败: {}", e)
        })?.path();

        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            debug!("[read_session_metadata] 跳过非 json 文件: {}", file_path.display());
            continue;
        }

        checked_files += 1;
        debug!("[read_session_metadata] 检查文件 #{}: {}", checked_files, file_path.display());

        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(metadata) = serde_json::from_str::<SessionMetadata>(&content) {
                parse_success += 1;
                debug!("[read_session_metadata] 文件 {} 的 sessionId={}", file_path.display(), metadata.session_id);

                if metadata.session_id == session_id {
                    let elapsed = start.elapsed();
                    info!("[read_session_metadata] 找到匹配: file={}, pid={}, cwd={}, started_at={}, status={}, 耗时: {}ms",
                          file_path.display(), metadata.pid, metadata.cwd, metadata.started_at, metadata.status, elapsed.as_millis());
                    return Ok(metadata);
                }
            } else {
                parse_fail += 1;
                warn!("[read_session_metadata] 解析失败: {}", file_path.display());
            }
        } else {
            warn!("[read_session_metadata] 读取文件失败: {}", file_path.display());
        }
    }

    let elapsed = start.elapsed();
    warn!("[read_session_metadata] 未找到 session_id={}, 检查了 {} 个文件, 解析成功={}, 解析失败={}, 耗时: {}ms",
          session_id, checked_files, parse_success, parse_fail, elapsed.as_millis());
    Err(format!("未找到 session_id={} 的元数据", session_id))
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
    info!("[add_running_session] 开始添加 session: {}", session_id);
    let start = Instant::now();

    // 读取 session 元数据获取 PID
    debug!("[add_running_session] 读取 session 元数据");
    let metadata = match read_session_metadata(session_id) {
        Ok(m) => {
            info!("[add_running_session] 元数据读取成功: pid={}, cwd={}", m.pid, m.cwd);
            m
        }
        Err(e) => {
            error!("[add_running_session] 元数据读取失败: {}", e);
            return Err(e);
        }
    };

    // 验证进程是否为 claude
    debug!("[add_running_session] 检查进程 PID={} 是否为 claude", metadata.pid);
    if !is_claude_process_running(metadata.pid) {
        warn!("[add_running_session] PID {} 不是 claude 进程或进程已退出", metadata.pid);
        return Err(format!("PID {} 不是 claude 进程", metadata.pid));
    }
    info!("[add_running_session] PID {} 确认是 claude 进程", metadata.pid);

    // 提取名称
    let name = get_path_name(&metadata.cwd);
    debug!("[add_running_session] 提取名称: {} -> {}", metadata.cwd, name);

    // 确定状态
    let status = if metadata.status == "idle" {
        debug!("[add_running_session] 元数据 status=idle -> WaitingInput");
        SessionStatus::WaitingInput
    } else {
        debug!("[add_running_session] 元数据 status={} -> Running", metadata.status);
        SessionStatus::Running
    };

    info!("[add_running_session] 创建 RunningSession: id={}, pid={}, status={}, cwd={}, name={}",
          session_id, metadata.pid,
          if status == SessionStatus::WaitingInput { "waiting_input" } else { "running" },
          metadata.cwd, name);

    let session = RunningSession {
        session_id: session_id.to_string(),
        pid: metadata.pid,
        status,
        cwd: metadata.cwd,
        name,
        updated_at: metadata.updated_at.unwrap_or(metadata.started_at),
    };

    // 添加到全局状态
    debug!("[add_running_session] 获取锁并添加到 RUNNING_SESSIONS");
    RUNNING_SESSIONS.lock().unwrap().insert(session_id.to_string(), session);

    let elapsed = start.elapsed();
    info!("[add_running_session] 完成，耗时: {}ms", elapsed.as_millis());
    Ok(())
}

/// 更新 session 状态
pub fn update_session_status(session_id: &str, status: SessionStatus) {
    info!("[update_session_status] 开始更新: session_id={}, 目标状态={}",
          session_id, if status == SessionStatus::WaitingInput { "waiting_input" } else { "running" });

    let mut sessions = RUNNING_SESSIONS.lock().unwrap();

    if let Some(session) = sessions.get_mut(session_id) {
        let old_status = session.status.clone();
        debug!("[update_session_status] 当前状态: {}", if old_status == SessionStatus::WaitingInput { "waiting_input" } else { "running" });

        session.status = status;
        session.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        info!("[update_session_status] 完成: {} -> {}",
              if old_status == SessionStatus::WaitingInput { "waiting_input" } else { "running" },
              if status == SessionStatus::WaitingInput { "waiting_input" } else { "running" });
    } else {
        warn!("[update_session_status] 未找到 session: {}", session_id);
    }
}

/// 移除运行中 session
pub fn remove_running_session(session_id: &str) {
    info!("[remove_running_session] 开始移除: {}", session_id);

    let mut sessions = RUNNING_SESSIONS.lock().unwrap();

    if sessions.remove(session_id).is_some() {
        info!("[remove_running_session] 完成: {} 已移除，剩余 {} 个 session",
              session_id, sessions.len());
    } else {
        warn!("[remove_running_session] 未找到 session: {}", session_id);
    }
}

/// 获取所有运行中 session
pub fn get_running_sessions() -> Vec<RunningSession> {
    debug!("[get_running_sessions] 开始获取");

    let sessions: Vec<RunningSession> = RUNNING_SESSIONS.lock().unwrap().values().cloned().collect();

    let running_count = sessions.iter().filter(|s| s.status == SessionStatus::Running).count();
    let waiting_count = sessions.iter().filter(|s| s.status == SessionStatus::WaitingInput).count();

    debug!("[get_running_sessions] 完成: 总数={}, running={}, waiting_input={}",
           sessions.len(), running_count, waiting_count);
    sessions
}

/// 应用启动时初始化运行中 session 列表
pub fn init_running_sessions() -> Result<Vec<RunningSession>, String> {
    info!("[init_running_sessions] 开始初始化运行中 session 列表");
    let start = Instant::now();

    let events_dir = get_events_dir();
    debug!("[init_running_sessions] 事件目录: {}", events_dir.display());

    if !events_dir.exists() {
        info!("[init_running_sessions] 事件目录不存在，创建目录");
        fs::create_dir_all(&events_dir)
            .map_err(|e| {
                error!("[init_running_sessions] 创建事件目录失败: {}", e);
                format!("创建事件目录失败: {}", e)
            })?;
        info!("[init_running_sessions] 事件目录创建成功，返回空列表");
        return Ok(Vec::new());
    }

    // 读取所有事件文件
    info!("[init_running_sessions] 读取事件文件");
    let mut events_by_session: HashMap<String, Vec<HookEvent>> = HashMap::new();
    let mut total_files = 0;
    let mut json_files = 0;
    let mut parsed_files = 0;
    let mut parse_failed = 0;

    let entries = fs::read_dir(&events_dir)
        .map_err(|e| {
            error!("[init_running_sessions] 读取事件目录失败: {}", e);
            format!("读取事件目录失败: {}", e)
        })?;

    for entry in entries {
        let file_path = entry.map_err(|e| {
            warn!("[init_running_sessions] 读取条目失败: {}", e);
            format!("读取条目失败: {}", e)
        })?.path();

        total_files += 1;

        if file_path.extension().and_then(|s| s.to_str()) != Some("json") {
            debug!("[init_running_sessions] 跳过非 json 文件: {}", file_path.display());
            continue;
        }

        json_files += 1;
        debug!("[init_running_sessions] 读取事件文件 #{}: {}", json_files, file_path.display());

        if let Ok(event) = parse_hook_event(&file_path) {
            parsed_files += 1;
            debug!("[init_running_sessions] 解析成功: session_id={}, event_type={}",
                   event.session_id, event.hook_event_name);
            events_by_session
                .entry(event.session_id.clone())
                .or_insert_with(Vec::new)
                .push(event);
        } else {
            parse_failed += 1;
            warn!("[init_running_sessions] 解析失败: {}", file_path.display());
        }
    }

    info!("[init_running_sessions] 文件统计: 总数={}, json={}, 解析成功={}, 解析失败={}",
          total_files, json_files, parsed_files, parse_failed);
    info!("[init_running_sessions] 按 session 分组: {} 个不同 session", events_by_session.len());

    // 清空现有状态
    debug!("[init_running_sessions] 清空 RUNNING_SESSIONS");
    RUNNING_SESSIONS.lock().unwrap().clear();

    // 分析每个 session 的状态
    let mut added_count = 0;
    let mut skipped_end_count = 0;
    let mut skipped_no_start_count = 0;
    let mut add_failed_count = 0;

    for (session_id, events) in events_by_session.iter() {
        info!("[init_running_sessions] 分析 session {}，事件数量: {}", session_id, events.len());

        // 打印所有事件类型
        for event in events {
            debug!("[init_running_sessions] session {} 事件: type={}, cwd={}, model={}",
                   session_id, event.hook_event_name,
                   event.cwd.as_ref().unwrap_or(&"none".to_string()),
                   event.model.as_ref().unwrap_or(&"none".to_string()));
        }

        // 检查是否有 SessionEnd
        let has_end = events.iter().any(|e| e.hook_event_name == "SessionEnd");
        if has_end {
            skipped_end_count += 1;
            info!("[init_running_sessions] session {} 有 SessionEnd，跳过", session_id);
            continue;
        }

        // 检查是否有 SessionStart
        let has_start = events.iter().any(|e| e.hook_event_name == "SessionStart");
        if has_start {
            info!("[init_running_sessions] session {} 有 SessionStart，尝试添加", session_id);
            // 尝试添加到运行中列表
            if add_running_session(session_id).is_ok() {
                added_count += 1;
                // 检查是否有 Notification（等待输入）
                let has_notification = events.iter().any(|e| e.hook_event_name == "Notification");
                if has_notification {
                    info!("[init_running_sessions] session {} 有 Notification，更新为 waiting_input", session_id);
                    update_session_status(session_id, SessionStatus::WaitingInput);
                }
            } else {
                add_failed_count += 1;
                warn!("[init_running_sessions] 添加 session {} 失败", session_id);
            }
        } else {
            skipped_no_start_count += 1;
            debug!("[init_running_sessions] session {} 没有 SessionStart，跳过", session_id);
        }
    }

    info!("[init_running_sessions] 分析结果: 添加={}, SessionEnd跳过={}, 无SessionStart跳过={}, 添加失败={}",
          added_count, skipped_end_count, skipped_no_start_count, add_failed_count);

    // 清理已处理的事件文件
    info!("[init_running_sessions] 清理事件目录");
    cleanup_events_dir(&events_dir);

    let result = get_running_sessions();
    let elapsed = start.elapsed();
    info!("[init_running_sessions] 完成，运行中 session 数量: {}，耗时: {}ms", result.len(), elapsed.as_millis());
    Ok(result)
}

/// 清理事件目录
fn cleanup_events_dir(events_dir: &PathBuf) {
    info!("[cleanup_events_dir] 开始清理: {}", events_dir.display());
    let start = Instant::now();

    if let Ok(entries) = fs::read_dir(events_dir) {
        let mut cleaned = 0;
        let mut failed = 0;

        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.is_file() {
                debug!("[cleanup_events_dir] 删除文件: {}", file_path.display());
                if fs::remove_file(&file_path).is_ok() {
                    cleaned += 1;
                } else {
                    failed += 1;
                    warn!("[cleanup_events_dir] 删除失败: {}", file_path.display());
                }
            }
        }

        let elapsed = start.elapsed();
        info!("[cleanup_events_dir] 完成: 清理={}, 失败={}, 耗时: {}ms", cleaned, failed, elapsed.as_millis());
    } else {
        warn!("[cleanup_events_dir] 无法读取目录");
    }
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

            // 获取当前 session 数量
            let session_count = RUNNING_SESSIONS.lock().unwrap().len();
            debug!("[polling_thread] 当前 session 数量: {}", session_count);

            if session_count == 0 {
                debug!("[polling_thread] 无 session，跳过进程检查");
                continue;
            }

            // 检查每个 session 的进程状态
            let mut checked_pids: Vec<(String, u32)> = Vec::new();
            let sessions_to_remove: Vec<String> = RUNNING_SESSIONS
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, session)| {
                    checked_pids.push((session.session_id.clone(), session.pid));
                    let running = is_claude_process_running(session.pid);
                    if !running {
                        warn!("[polling_thread] PID {} (session {}) 已退出", session.pid, session.session_id);
                    }
                    !running
                })
                .map(|(id, _)| id.clone())
                .collect();

            debug!("[polling_thread] 检查了 {} 个 PID: {:?}", checked_pids.len(), checked_pids);

            let removed_count = sessions_to_remove.len();
            if removed_count > 0 {
                info!("[polling_thread] 检测到 {} 个意外退出的 session", removed_count);
            }

            for session_id in &sessions_to_remove {
                info!("[polling_thread] 移除 session: {}", session_id);
                RUNNING_SESSIONS.lock().unwrap().remove(session_id);
            }

            // 通知前端
            let sessions = get_running_sessions();
            let new_count = sessions.len();

            debug!("[polling_thread] 发送 running_sessions_changed 事件，数量: {} -> {}", session_count, new_count);
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