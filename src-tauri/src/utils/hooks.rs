use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Emitter;
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

/// 确保 hook_writer.py 脚本存在
pub fn ensure_hook_writer() -> Result<(), String> {
    let claude_fleet_dir = get_claude_fleet_dir();
    let script_path = claude_fleet_dir.join("hook_writer.py");

    // 创建目录
    fs::create_dir_all(&claude_fleet_dir)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    // 脚本内容
    let script_content = r#"import os, sys, json
from datetime import datetime

hook_input = json.load(sys.stdin)
session_id = hook_input.get("session_id", "unknown")
timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
events_dir = os.path.expanduser("~/.claude-fleet/events")
os.makedirs(events_dir, exist_ok=True)
file_path = os.path.join(events_dir, f"{session_id}_{timestamp}.json")
with open(file_path, "w", encoding="utf-8") as f:
    json.dump(hook_input, f, indent=2, ensure_ascii=False)
"#;

    // 写入脚本（不存在时）
    if !script_path.exists() {
        fs::write(&script_path, script_content)
            .map_err(|e| format!("写入脚本失败: {}", e))?;
    }

    Ok(())
}

/// 清理事件目录中的所有文件
pub fn cleanup_events_dir() -> Result<(), String> {
    let events_dir = get_events_dir();

    if events_dir.exists() {
        // 删除目录中的所有文件，但保留目录本身
        for entry in fs::read_dir(&events_dir)
            .map_err(|e| format!("读取事件目录失败: {}", e))?
        {
            let file_path = entry
                .map_err(|e| format!("读取条目失败: {}", e))?
                .path();

            if file_path.is_file() {
                fs::remove_file(&file_path)
                    .map_err(|e| format!("删除文件 {} 失败: {}", file_path.display(), e))?;
            }
        }
    } else {
        // 创建目录
        fs::create_dir_all(&events_dir)
            .map_err(|e| format!("创建事件目录失败: {}", e))?;
    }

    Ok(())
}

/// 启动钩子事件接收服务（文件监听方式）
pub fn start_hook_receiver(app_handle: tauri::AppHandle) -> Result<(), String> {
    if HOOK_RECEIVER_RUNNING.load(Ordering::SeqCst) {
        return Ok(());
    }

    HOOK_RECEIVER_RUNNING.store(true, Ordering::SeqCst);

    let events_dir = get_events_dir();

    // 启动时清理历史文件
    cleanup_events_dir()?;

    // 处理事件的后台线程（将 watcher 移入线程以保持存活）
    let app_handle_clone = app_handle.clone();
    let events_dir_clone = events_dir.clone();

    thread::spawn(move || {
        // 在线程内创建监听器，确保 watcher 生命周期与线程同步
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: RecommendedWatcher = match notify::recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("创建文件监听器失败: {}", e);
                HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
                return;
            }
        };

        // 开始监听事件目录
        if let Err(e) = watcher.watch(&events_dir_clone, RecursiveMode::NonRecursive) {
            eprintln!("监听事件目录失败: {}", e);
            HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
            return;
        }

        loop {
            if !HOOK_RECEIVER_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            // 接收文件系统事件
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(result) => {
                    // result 是 Result<Event, Error>
                    if let Ok(event) = result {
                        process_file_event(&event, &app_handle_clone);
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 继续循环
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }

        // 退出时清理
        cleanup_events_dir_on_exit(&events_dir_clone);
    });

    Ok(())
}

/// 处理文件系统事件
fn process_file_event(event: &Event, app_handle: &tauri::AppHandle) {
    // 只处理修改事件（文件写入完成后触发，避免读取空文件）
    match event.kind {
        EventKind::Modify(_) => {}
        _ => return,
    }

    for path in &event.paths {
        // 只处理 JSON 文件
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        // 延迟一小段时间等待文件写入完成
        thread::sleep(Duration::from_millis(100));

        // 解析 hook 事件
        if let Ok(hook_event) = parse_hook_event(path) {
            // 增量更新状态
            handle_hook_event_incremental(&hook_event, app_handle);
        }

        // 删除已处理的文件（防止堆积）
        if let Err(e) = fs::remove_file(path) {
            eprintln!("删除事件文件 {} 失败: {}", path.display(), e);
        }
    }
}

/// 增量处理 hook 事件
fn handle_hook_event_incremental(event: &HookEvent, app_handle: &tauri::AppHandle) {
    println!("处理 hook 事件: {} - {}", event.hook_event_name, event.session_id);

    match event.hook_event_name.as_str() {
        "SessionStart" => {
            // 添加到运行中列表
            if add_running_session(&event.session_id).is_ok() {
                emit_sessions_changed(app_handle);
            }
        }
        "Notification" => {
            // 更新为等待输入状态
            update_session_status(&event.session_id, SessionStatus::WaitingInput);
            emit_sessions_changed(app_handle);

            // 发送通知事件（供前端发送桌面通知）
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

/// 退出时清理事件目录
fn cleanup_events_dir_on_exit(events_dir: &PathBuf) {
    if events_dir.exists() {
        // 删除目录中的所有文件
        if let Ok(entries) = fs::read_dir(events_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    if file_path.is_file() {
                        fs::remove_file(&file_path).ok();
                    }
                }
            }
        }
        // 不删除目录本身，以便下次启动时可以直接使用
    }
}

/// 停止钩子接收服务
pub fn stop_hook_receiver() {
    HOOK_RECEIVER_RUNNING.store(false, Ordering::SeqCst);
}

/// 处理钩子事件（供外部调用）
pub fn handle_hook_event(event: HookEvent) -> Result<(), String> {
    // 此函数可用于扩展处理逻辑
    let _ = event;
    Ok(())
}

/// 手动触发钩子事件（用于测试）
pub fn trigger_hook_event(app_handle: &tauri::AppHandle, event: HookEvent) -> Result<(), String> {
    app_handle.emit("hook_event", &event)
        .map_err(|e| format!("发送事件失败: {}", e))
}

/// 获取事件目录路径（供外部查询）
pub fn get_events_dir_path() -> String {
    get_events_dir().to_string_lossy().to_string()
}