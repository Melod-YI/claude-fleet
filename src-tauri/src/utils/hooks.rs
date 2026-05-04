use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Emitter;
use tracing::{info, debug, warn, error};
use std::time::Instant;
use crate::utils::running_sessions::{
    add_running_session,
    update_session_status,
    remove_running_session,
    get_running_sessions,
    parse_hook_event,
    SessionStatus,
    HookEvent,
};

static HOOK_RECEIVER_RUNNING: AtomicBool = AtomicBool::new(false);

/// 获取 Claude Fleet 数据目录路径
fn get_claude_fleet_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
}

/// 获取事件目录路径
fn get_events_dir() -> PathBuf {
    get_claude_fleet_dir().join("events")
}

/// 清理事件目录中的所有文件
pub fn cleanup_events_dir() -> Result<(), String> {
    info!("[cleanup_events_dir] 开始清理事件目录");
    let start = Instant::now();

    let events_dir = get_events_dir();
    debug!("[cleanup_events_dir] 事件目录: {}", events_dir.display());

    if events_dir.exists() {
        let mut cleaned = 0;
        let mut failed = 0;

        // 删除目录中的所有文件，但保留目录本身
        let entries = fs::read_dir(&events_dir)
            .map_err(|e| {
                error!("[cleanup_events_dir] 读取事件目录失败: {}", e);
                format!("读取事件目录失败: {}", e)
            })?;

        for entry in entries {
            let file_path = entry
                .map_err(|e| {
                    warn!("[cleanup_events_dir] 读取条目失败: {}", e);
                    format!("读取条目失败: {}", e)
                })?
                .path();

            if file_path.is_file() {
                debug!("[cleanup_events_dir] 删除文件: {}", file_path.display());
                if let Err(e) = fs::remove_file(&file_path) {
                    failed += 1;
                    warn!("[cleanup_events_dir] 删除文件 {} 失败: {}", file_path.display(), e);
                } else {
                    cleaned += 1;
                }
            }
        }

        let elapsed = start.elapsed();
        info!("[cleanup_events_dir] 完成，清理了 {} 个文件，失败 {} 个，耗时: {}ms",
              cleaned, failed, elapsed.as_millis());
    } else {
        // 创建目录
        debug!("[cleanup_events_dir] 目录不存在，创建目录");
        fs::create_dir_all(&events_dir)
            .map_err(|e| {
                error!("[cleanup_events_dir] 创建事件目录失败: {}", e);
                format!("创建事件目录失败: {}", e)
            })?;
        info!("[cleanup_events_dir] 创建事件目录: {}", events_dir.display());
    }

    Ok(())
}

/// 启动钩子事件接收服务（文件监听方式）
pub fn start_hook_receiver(app_handle: tauri::AppHandle) -> Result<(), String> {
    info!("[start_hook_receiver] 开始启动 hook 接收服务");
    let start = Instant::now();

    if HOOK_RECEIVER_RUNNING.load(Ordering::SeqCst) {
        warn!("[start_hook_receiver] hook 接收服务已在运行，跳过启动");
        return Ok(());
    }

    info!("[start_hook_receiver] 设置运行标志为 true");
    HOOK_RECEIVER_RUNNING.store(true, Ordering::SeqCst);

    let events_dir = get_events_dir();
    info!("[start_hook_receiver] 事件目录: {}", events_dir.display());

    // 启动时清理历史文件
    debug!("[start_hook_receiver] 调用 cleanup_events_dir");
    cleanup_events_dir()?;

    // 处理事件的后台线程（将 watcher 移入线程以保持存活）
    let app_handle_clone = app_handle.clone();
    let events_dir_clone = events_dir.clone();

    thread::spawn(move || {
        info!("[hook_thread] hook 监听线程启动");

        // 在线程内创建监听器，确保 watcher 生命周期与线程同步
        let (tx, rx) = std::sync::mpsc::channel();
        debug!("[hook_thread] 创建 channel");

        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(tx) {
            Ok(w) => {
                info!("[hook_thread] 创建文件监听器成功");
                w
            }
            Err(e) => {
                error!("[hook_thread] 创建文件监听器失败: {}", e);
                HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        // 开始监听事件目录
        if let Err(e) = watcher.watch(&events_dir_clone, RecursiveMode::NonRecursive) {
            error!("[hook_thread] 监听事件目录失败: {}", e);
            HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
            return;
        }
        info!("[hook_thread] 开始监听事件目录: {}", events_dir_clone.display());

        let mut event_count = 0;
        let mut modify_count = 0;
        let mut other_count = 0;

        loop {
            if !HOOK_RECEIVER_RUNNING.load(Ordering::SeqCst) {
                info!("[hook_thread] 收到停止信号，退出线程");
                break;
            }

            // 接收文件系统事件
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(result) => {
                    // result 是 Result<Event, Error>
                    if let Ok(event) = result {
                        event_count += 1;
                        debug!("[hook_thread] 收到文件系统事件 #{}: kind={:?}, paths={}",
                               event_count, event.kind, event.paths.len());
                        process_file_event(&event, &app_handle_clone);
                    } else {
                        warn!("[hook_thread] 文件系统事件错误: {:?}", result);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 继续循环，超时是正常的
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    warn!("[hook_thread] 文件监听器断开连接");
                    break;
                }
            }
        }

        info!("[hook_thread] 线程退出，共处理 {} 个事件（modify={}, other={})",
              event_count, modify_count, other_count);
        // 退出时清理
        cleanup_events_dir_on_exit(&events_dir_clone);
    });

    let elapsed = start.elapsed();
    info!("[start_hook_receiver] 完成，耗时: {}ms", elapsed.as_millis());
    Ok(())
}

/// 处理文件系统事件
fn process_file_event(event: &Event, app_handle: &tauri::AppHandle) {
    // 只处理修改事件（文件写入完成后触发，避免读取空文件）
    match event.kind {
        EventKind::Modify(_) => {
            debug!("[process_file_event] 处理 Modify 事件");
        }
        _ => {
            debug!("[process_file_event] 忽略非 Modify 事件: kind={:?}", event.kind);
            return;
        }
    }

    debug!("[process_file_event] 事件包含 {} 个路径", event.paths.len());

    for path in &event.paths {
        // 只处理 JSON 文件
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            debug!("[process_file_event] 忽略非 JSON 文件: {}", path.display());
            continue;
        }

        info!("[process_file_event] 检测到事件文件: {}", path.display());

        // 延迟一小段时间等待文件写入完成
        debug!("[process_file_event] 延迟 100ms 等待文件写入完成");
        thread::sleep(Duration::from_millis(100));

        // 解析 hook 事件
        debug!("[process_file_event] 开始解析事件文件");
        let parse_start = Instant::now();

        if let Ok(hook_event) = parse_hook_event(path) {
            let elapsed = parse_start.elapsed();
            info!("[process_file_event] 解析成功: type={}, session_id={}, cwd={}, 耗时: {}ms",
                  hook_event.hook_event_name, hook_event.session_id,
                  hook_event.cwd.as_ref().unwrap_or(&"none".to_string()), elapsed.as_millis());
            // 增量更新状态
            handle_hook_event_incremental(&hook_event, app_handle);
        } else {
            error!("[process_file_event] 解析事件文件失败: {}", path.display());
        }

        // 删除已处理的文件（防止堆积）
        debug!("[process_file_event] 删除已处理的文件: {}", path.display());
        if let Err(e) = fs::remove_file(path) {
            warn!("[process_file_event] 删除事件文件 {} 失败: {}", path.display(), e);
        } else {
            debug!("[process_file_event] 文件删除成功");
        }
    }
}

/// 增量处理 hook 事件
fn handle_hook_event_incremental(event: &HookEvent, app_handle: &tauri::AppHandle) {
    info!("[handle_hook_event_incremental] 开始处理: type={}, session_id={}",
          event.hook_event_name, event.session_id);
    debug!("[handle_hook_event_incremental] 事件详情: cwd={}, transcript_path={}, source={}, model={}, reason={}",
           event.cwd.as_ref().unwrap_or(&"none".to_string()),
           event.transcript_path.as_ref().unwrap_or(&"none".to_string()),
           event.source.as_ref().unwrap_or(&"none".to_string()),
           event.model.as_ref().unwrap_or(&"none".to_string()),
           event.reason.as_ref().unwrap_or(&"none".to_string()));

    let start = Instant::now();

    match event.hook_event_name.as_str() {
        "SessionStart" => {
            info!("[handle_hook_event_incremental] SessionStart 分支: 添加 session {}", event.session_id);
            // 添加到运行中列表
            match add_running_session(&event.session_id) {
                Ok(_) => {
                    info!("[handle_hook_event_incremental] session {} 添加成功", event.session_id);
                    emit_sessions_changed(app_handle);
                }
                Err(e) => {
                    warn!("[handle_hook_event_incremental] session {} 添加失败: {}", event.session_id, e);
                }
            }
        }
        "Notification" => {
            info!("[handle_hook_event_incremental] Notification 分支: session {} 等待输入", event.session_id);
            // 更新为等待输入状态
            update_session_status(&event.session_id, SessionStatus::WaitingInput);
            emit_sessions_changed(app_handle);

            // 发送通知事件（供前端发送桌面通知）
            debug!("[handle_hook_event_incremental] 发送 session_waiting_input 事件到前端");
            if let Err(e) = app_handle.emit("session_waiting_input", event) {
                error!("[handle_hook_event_incremental] 发送 session_waiting_input 事件失败: {}", e);
            } else {
                info!("[handle_hook_event_incremental] session_waiting_input 事件发送成功");
            }
        }
        "Stop" => {
            info!("[handle_hook_event_incremental] Stop 分支: session {} 响应完成", event.session_id);
            // 更新为运行状态
            update_session_status(&event.session_id, SessionStatus::Running);
            emit_sessions_changed(app_handle);
        }
        "SessionEnd" => {
            info!("[handle_hook_event_incremental] SessionEnd 分支: 移除 session {}", event.session_id);
            // 从运行中列表移除
            remove_running_session(&event.session_id);
            emit_sessions_changed(app_handle);
        }
        _ => {
            warn!("[handle_hook_event_incremental] 未知事件类型分支: {}", event.hook_event_name);
        }
    }

    let elapsed = start.elapsed();
    info!("[handle_hook_event_incremental] 完成，耗时: {}ms", elapsed.as_millis());
}

/// 发送状态变化事件
fn emit_sessions_changed(app_handle: &tauri::AppHandle) {
    debug!("[emit_sessions_changed] 开始发送状态变化事件");

    let sessions = get_running_sessions();
    let session_count = sessions.len();
    debug!("[emit_sessions_changed] 当前 session 数量: {}", session_count);

    if session_count > 0 {
        for session in &sessions {
            debug!("[emit_sessions_changed] session: id={}, pid={}, status={}, cwd={}",
                   session.session_id, session.pid,
                   if session.status == SessionStatus::WaitingInput { "waiting_input" } else { "running" },
                   session.cwd);
        }
    }

    if let Err(e) = app_handle.emit("running_sessions_changed", sessions) {
        error!("[emit_sessions_changed] 发送 running_sessions_changed 事件失败: {}", e);
    } else {
        info!("[emit_sessions_changed] running_sessions_changed 事件发送成功，数量: {}", session_count);
    }
}

/// 退出时清理事件目录
fn cleanup_events_dir_on_exit(events_dir: &PathBuf) {
    info!("[cleanup_events_dir_on_exit] 开始退出清理: {}", events_dir.display());
    let start = Instant::now();

    if events_dir.exists() {
        let mut cleaned = 0;
        let mut failed = 0;

        // 删除目录中的所有文件
        if let Ok(entries) = fs::read_dir(events_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    if file_path.is_file() {
                        debug!("[cleanup_events_dir_on_exit] 删除文件: {}", file_path.display());
                        if let Err(e) = fs::remove_file(&file_path) {
                            failed += 1;
                            warn!("[cleanup_events_dir_on_exit] 删除失败: {} - {}", file_path.display(), e);
                        } else {
                            cleaned += 1;
                        }
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        info!("[cleanup_events_dir_on_exit] 完成，清理 {} 个文件，失败 {} 个，耗时: {}ms",
              cleaned, failed, elapsed.as_millis());
    } else {
        debug!("[cleanup_events_dir_on_exit] 目录不存在，无需清理");
    }
    // 不删除目录本身，以便下次启动时可以直接使用
}

/// 停止钩子接收服务
pub fn stop_hook_receiver() {
    info!("[stop_hook_receiver] 开始停止 hook 接收服务");
    HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
    info!("[stop_hook_receiver] 运行标志已设置为 false");
}

/// 处理钩子事件（供外部调用）
pub fn handle_hook_event(event: HookEvent) -> Result<(), String> {
    info!("[handle_hook_event] 外部调用处理事件: type={}, session_id={}",
          event.hook_event_name, event.session_id);
    // 此函数可用于扩展处理逻辑
    let _ = event;
    Ok(())
}

/// 手动触发钩子事件（用于测试）
pub fn trigger_hook_event(app_handle: &tauri::AppHandle, event: HookEvent) -> Result<(), String> {
    info!("[trigger_hook_event] 手动触发事件: type={}, session_id={}",
          event.hook_event_name, event.session_id);

    app_handle.emit("hook_event", &event)
        .map_err(|e| {
            error!("[trigger_hook_event] 发送事件失败: {}", e);
            format!("发送事件失败: {}", e)
        })?;

    info!("[trigger_hook_event] 事件发送成功");
    Ok(())
}

/// 获取事件目录路径（供外部查询）
pub fn get_events_dir_path() -> String {
    let path = get_events_dir().to_string_lossy().to_string();
    debug!("[get_events_dir_path] 返回: {}", path);
    path
}