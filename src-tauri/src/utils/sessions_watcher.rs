use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashSet;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use tauri::Emitter;
use tracing::{info, debug, warn, error};
use crate::utils::running_sessions::{
    add_running_session_from_file,
    update_session_status_from_file,
    remove_running_session_by_pid,
    get_running_sessions,
    scan_session_jsonl_force,
    SessionStatus,
    SessionFileContent,
};

static WATCHER_RUNNING: AtomicBool = AtomicBool::new(false);

/// 启动时间（用于避免启动初期发送通知）
static WATCHER_START_TIME: Lazy<Mutex<Option<Instant>>> = Lazy::new(|| Mutex::new(None));

/// 启动后多久才开始发送通知（秒）
const NOTIFICATION_DELAY_SECS: u64 = 5;

/// 已处理文件的记录（用于去重，避免同一文件被处理多次）
static PROCESSED_FILES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// 获取 sessions 目录路径
fn get_sessions_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude")
        .join("sessions")
}

/// 从文件名解析 PID："33804.json" -> 33804
fn parse_pid_from_filename(filename: &str) -> Result<u32, String> {
    filename
        .strip_suffix(".json")
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| format!("无法从文件名解析 PID: {}", filename))
}

/// 解析 session 文件内容
fn parse_session_file(file_path: &PathBuf) -> Result<SessionFileContent, String> {
    info!("[parse_session_file] 开始解析: {}", file_path.display());
    let start = Instant::now();

    let content = fs::read_to_string(file_path)
        .map_err(|e| {
            error!("[parse_session_file] 读取失败: {}", e);
            format!("读取文件失败: {}", e)
        })?;

    debug!("[parse_session_file] 文件内容长度: {} 字节", content.len());

    let session: SessionFileContent = serde_json::from_str(&content)
        .map_err(|e| {
            error!("[parse_session_file] JSON 解析失败: {}", e);
            format!("解析 JSON 失败: {}", e)
        })?;

    let elapsed = start.elapsed();
    info!("[parse_session_file] 完成: pid={}, sessionId={}, status={}, 耗时: {}ms",
          session.pid, session.session_id, session.status, elapsed.as_millis());
    Ok(session)
}

/// 启动 sessions 目录监听服务
pub fn start_sessions_watcher(app_handle: tauri::AppHandle) -> Result<(), String> {
    info!("[start_sessions_watcher] 开始启动 sessions 监听服务");
    let start = Instant::now();

    if WATCHER_RUNNING.load(Ordering::SeqCst) {
        warn!("[start_sessions_watcher] 监听服务已在运行，跳过启动");
        return Ok(());
    }

    info!("[start_sessions_watcher] 设置运行标志为 true");
    WATCHER_RUNNING.store(true, Ordering::SeqCst);

    // 记录启动时间（用于避免启动初期发送通知）
    {
        let mut start_time = WATCHER_START_TIME.lock().unwrap();
        *start_time = Some(Instant::now());
    }
    info!("[start_sessions_watcher] 记录启动时间，通知将在 {} 秒后开始发送", NOTIFICATION_DELAY_SECS);

    let sessions_dir = get_sessions_dir();
    info!("[start_sessions_watcher] sessions 目录: {}", sessions_dir.display());

    // 确保目录存在
    if !sessions_dir.exists() {
        debug!("[start_sessions_watcher] 目录不存在，创建目录");
        fs::create_dir_all(&sessions_dir)
            .map_err(|e| {
                error!("[start_sessions_watcher] 创建目录失败: {}", e);
                format!("创建 sessions 目录失败: {}", e)
            })?;
    }

    // 处理事件的后台线程
    let app_handle_clone = app_handle.clone();
    let sessions_dir_clone = sessions_dir.clone();

    thread::spawn(move || {
        info!("[sessions_watcher_thread] 监听线程启动");

        let (tx, rx) = std::sync::mpsc::channel();
        debug!("[sessions_watcher_thread] 创建 channel");

        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(tx) {
            Ok(w) => {
                info!("[sessions_watcher_thread] 创建文件监听器成功");
                w
            }
            Err(e) => {
                error!("[sessions_watcher_thread] 创建文件监听器失败: {}", e);
                WATCHER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        if let Err(e) = watcher.watch(&sessions_dir_clone, RecursiveMode::NonRecursive) {
            error!("[sessions_watcher_thread] 监听目录失败: {}", e);
            WATCHER_RUNNING.store(false, Ordering::SeqCst);
            return;
        }
        info!("[sessions_watcher_thread] 开始监听目录: {}", sessions_dir_clone.display());

        let mut event_count = 0;

        loop {
            if !WATCHER_RUNNING.load(Ordering::SeqCst) {
                info!("[sessions_watcher_thread] 收到停止信号，退出线程");
                break;
            }

            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(result) => {
                    if let Ok(event) = result {
                        event_count += 1;
                        debug!("[sessions_watcher_thread] 收到事件 #{}: kind={:?}, paths={}",
                               event_count, event.kind, event.paths.len());
                        process_session_event(&event, &app_handle_clone);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 继续循环
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    warn!("[sessions_watcher_thread] 文件监听器断开连接");
                    break;
                }
            }
        }

        info!("[sessions_watcher_thread] 线程退出，共处理 {} 个事件", event_count);
    });

    let elapsed = start.elapsed();
    info!("[start_sessions_watcher] 完成，耗时: {}ms", elapsed.as_millis());
    Ok(())
}

/// 处理文件系统事件
fn process_session_event(event: &Event, app_handle: &tauri::AppHandle) {
    debug!("[process_session_event] 事件类型: {:?}", event.kind);

    for path in &event.paths {
        // 只处理 JSON 文件
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            debug!("[process_session_event] 忽略非 JSON 文件: {}", path.display());
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // 去重检查
        {
            let mut processed = PROCESSED_FILES.lock().unwrap();
            if processed.contains(&path_str) {
                debug!("[process_session_event] 文件已处理过，跳过: {}", path.display());
                continue;
            }
            processed.insert(path_str.clone());
        }

        // 判断事件类型
        let is_create = event.kind == EventKind::Create(notify::event::CreateKind::File)
            || event.kind == EventKind::Create(notify::event::CreateKind::Any);
        let is_modify = matches!(event.kind, EventKind::Modify(_));
        let is_remove = event.kind == EventKind::Remove(notify::event::RemoveKind::File)
            || event.kind == EventKind::Remove(notify::event::RemoveKind::Any);

        if is_create {
            info!("[process_session_event] 检测到文件创建: {}", path.display());
            handle_session_create(path, app_handle);
        } else if is_modify {
            info!("[process_session_event] 检测到文件修改: {}", path.display());
            handle_session_modify(path, app_handle);
        } else if is_remove {
            info!("[process_session_event] 检测到文件删除: {}", path.display());
            handle_session_remove(filename, app_handle);
        } else {
            debug!("[process_session_event] 忽略其他事件类型: {:?}", event.kind);
        }

        // 处理完成后从去重集合移除
        {
            let mut processed = PROCESSED_FILES.lock().unwrap();
            processed.remove(&path_str);
        }
    }
}

/// 处理 session 文件创建事件
fn handle_session_create(path: &PathBuf, app_handle: &tauri::AppHandle) {
    let start = Instant::now();

    // 延迟等待文件写入完成
    debug!("[handle_session_create] 延迟 100ms 等待文件写入完成");
    thread::sleep(Duration::from_millis(100));

    // 解析文件内容
    let session = match parse_session_file(path) {
        Ok(s) => s,
        Err(e) => {
            warn!("[handle_session_create] 解析失败: {}", e);
            return;
        }
    };

    // 添加到运行中列表
    if let Err(e) = add_running_session_from_file(&session) {
        warn!("[handle_session_create] 添加失败: {}", e);
        return;
    }

    info!("[handle_session_create] session 添加成功: pid={}, sessionId={}", session.pid, session.session_id);

    // 立即扫描 jsonl 获取 away_summary 和 last_user_input
    scan_session_jsonl_force(session.pid);

    // 发送状态变化事件
    emit_sessions_changed(app_handle);

    // 如果状态是 idle 或 waiting，发送通知事件（都是等待用户输入）
    if session.status == "idle" || session.status == "waiting" {
        debug!("[handle_session_create] 状态为 {}，发送通知事件", session.status);
        emit_waiting_input_notification(&session, app_handle);
    }

    let elapsed = start.elapsed();
    info!("[handle_session_create] 完成，耗时: {}ms", elapsed.as_millis());
}

/// 处理 session 文件修改事件
fn handle_session_modify(path: &PathBuf, app_handle: &tauri::AppHandle) {
    let start = Instant::now();

    // 延迟等待文件写入完成
    debug!("[handle_session_modify] 延迟 100ms 等待文件写入完成");
    thread::sleep(Duration::from_millis(100));

    // 解析文件内容
    let session = match parse_session_file(path) {
        Ok(s) => s,
        Err(e) => {
            warn!("[handle_session_modify] 解析失败: {}", e);
            return;
        }
    };

    // 更新状态（PID 作为查找 key）
    let old_status = get_session_status_by_pid(session.pid);
    update_session_status_from_file(&session);

    info!("[handle_session_modify] session 状态更新: pid={}, sessionId={}, status={}",
          session.pid, session.session_id, session.status);

    // 立即扫描 jsonl 获取 away_summary 和 last_user_input
    scan_session_jsonl_force(session.pid);

    // 发送状态变化事件
    emit_sessions_changed(app_handle);

    // 判断当前是否为等待输入状态
    let is_waiting_now = session.status == "idle" || session.status == "waiting";

    // 判断之前是否为等待输入状态
    let was_waiting_before = old_status == Some(SessionStatus::Idle)
        || old_status == Some(SessionStatus::Waiting);

    // 如果状态从非等待变为等待，发送通知事件
    if is_waiting_now && !was_waiting_before {
        debug!("[handle_session_modify] 状态变为 {}（等待输入），发送通知事件", session.status);
        emit_waiting_input_notification(&session, app_handle);
    }

    let elapsed = start.elapsed();
    info!("[handle_session_modify] 完成，耗时: {}ms", elapsed.as_millis());
}

/// 处理 session 文件删除事件
fn handle_session_remove(filename: &str, app_handle: &tauri::AppHandle) {
    let start = Instant::now();

    // 从文件名解析 PID
    let pid = match parse_pid_from_filename(filename) {
        Ok(p) => p,
        Err(e) => {
            warn!("[handle_session_remove] 解析 PID 失败: {}", e);
            return;
        }
    };

    // 根据 PID 移除 session
    remove_running_session_by_pid(pid);

    info!("[handle_session_remove] session 移除成功: pid={}", pid);

    // 发送状态变化事件
    emit_sessions_changed(app_handle);

    let elapsed = start.elapsed();
    info!("[handle_session_remove] 完成，耗时: {}ms", elapsed.as_millis());
}

/// 获取 session 当前状态（PID 作为 key）
fn get_session_status_by_pid(pid: u32) -> Option<SessionStatus> {
    use crate::utils::running_sessions::RUNNING_SESSIONS;
    RUNNING_SESSIONS.lock().unwrap().get(&pid).map(|s| s.status)
}

/// 发送状态变化事件
fn emit_sessions_changed(app_handle: &tauri::AppHandle) {
    debug!("[emit_sessions_changed] 开始发送状态变化事件");

    let sessions = get_running_sessions();
    let session_count = sessions.len();

    debug!("[emit_sessions_changed] 当前 session 数量: {}", session_count);

    if let Err(e) = app_handle.emit("running_sessions_changed", sessions) {
        error!("[emit_sessions_changed] 发送事件失败: {}", e);
    } else {
        info!("[emit_sessions_changed] 事件发送成功，数量: {}", session_count);
    }
}

/// 发送等待输入通知事件
fn emit_waiting_input_notification(session: &SessionFileContent, app_handle: &tauri::AppHandle) {
    use crate::utils::running_sessions::HookEvent;

    // 检查是否已过了启动延迟期（避免启动初期发送大量通知）
    {
        let start_time = WATCHER_START_TIME.lock().unwrap();
        if let Some(start) = *start_time {
            let elapsed = start.elapsed().as_secs();
            if elapsed < NOTIFICATION_DELAY_SECS {
                debug!("[emit_waiting_input_notification] 启动后 {} 秒，仍在延迟期内（需 {} 秒），跳过通知: sessionId={}",
                       elapsed, NOTIFICATION_DELAY_SECS, session.session_id);
                return;
            }
        }
    }

    // 构造事件数据（兼容前端现有格式）
    let event_data = HookEvent {
        session_id: session.session_id.clone(),
        hook_event_name: "Notification".to_string(),
        cwd: Some(session.cwd.clone()),
        transcript_path: None,
        source: None,
        model: None,
        reason: session.waiting_for.clone(),
    };

    if let Err(e) = app_handle.emit("session_waiting_input", event_data) {
        error!("[emit_waiting_input_notification] 发送事件失败: {}", e);
    } else {
        info!("[emit_waiting_input_notification] 事件发送成功: sessionId={}", session.session_id);
    }
}

/// 停止 sessions 监听服务
pub fn stop_sessions_watcher() {
    info!("[stop_sessions_watcher] 开始停止监听服务");
    WATCHER_RUNNING.store(false, Ordering::SeqCst);
    info!("[stop_sessions_watcher] 运行标志已设置为 false");
}