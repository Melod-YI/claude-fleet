use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

static HOOK_RECEIVER_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event: String,  // "start", "idle", "stop", "end"
    pub session_id: String,
    pub cwd: Option<String>,
}

/// 获取事件目录路径
fn get_events_dir() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("events")
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

    // 创建监听器
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: RecommendedWatcher = notify::recommended_watcher(tx)
        .map_err(|e| format!("创建文件监听器失败: {}", e))?;

    // 开始监听事件目录
    watcher.watch(&events_dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("监听事件目录失败: {}", e))?;

    // 处理事件的后台线程
    let app_handle_clone = app_handle.clone();
    let events_dir_clone = events_dir.clone();

    thread::spawn(move || {
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
    // 只处理文件创建事件
    if event.kind != EventKind::Create(notify::event::CreateKind::File) {
        return;
    }

    for path in &event.paths {
        // 只处理 JSON 文件
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        // 读取文件内容
        if let Ok(content) = fs::read_to_string(path) {
            // 解析 HookEvent
            if let Ok(hook_event) = serde_json::from_str::<HookEvent>(&content) {
                // 发送到前端
                if let Err(e) = app_handle.emit("hook_event", &hook_event) {
                    eprintln!("发送钩子事件失败: {}", e);
                }

                println!("收到钩子事件: {} - {}", hook_event.event, hook_event.session_id);
            }
        }

        // 立即删除已处理的文件（防止堆积）
        if let Err(e) = fs::remove_file(path) {
            eprintln!("删除事件文件 {} 失败: {}", path.display(), e);
        }
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
    match event.event.as_str() {
        "start" => {
            println!("Session started: {}", event.session_id);
        }
        "idle" => {
            // 等待用户输入 - 这是主要关注的事件
            println!("Session idle (waiting for input): {}", event.session_id);
        }
        "stop" => {
            // Claude 完成响应
            println!("Session stopped (response complete): {}", event.session_id);
        }
        "end" => {
            // Session 结束
            println!("Session ended: {}", event.session_id);
        }
        _ => {}
    }
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