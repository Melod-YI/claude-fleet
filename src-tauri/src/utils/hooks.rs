use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

static HOOK_SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_type: String,  // "session_start", "waiting_input", "session_end"
    pub session_id: String,
    pub working_directory: String,
    pub timestamp: String,
}

/// 启动钩子接收服务（轮询检查 session 状态变化）
pub fn start_hook_server(app_handle: tauri::AppHandle) -> Result<(), String> {
    if HOOK_SERVER_RUNNING.load(Ordering::SeqCst) {
        return Ok(())
    }

    HOOK_SERVER_RUNNING.store(true, Ordering::SeqCst);

    // 在后台线程启动轮询服务
    // 定期检查 session 文件变化，检测 waiting_input 状态
    thread::spawn(move || {
        let mut last_waiting_sessions: std::collections::HashSet<String> = std::collections::HashSet::new();

        loop {
            if !HOOK_SERVER_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            // 获取当前所有 session
            if let Ok(sessions) = crate::utils::claude_data::get_all_sessions() {
                let mut current_waiting: std::collections::HashSet<String> = std::collections::HashSet::new();

                for session in &sessions {
                    if session.status == "running" {
                        // 检查是否是新进入等待状态的 session
                        if !last_waiting_sessions.contains(&session.id) {
                            // 发送等待输入事件到前端
                            let event = HookEvent {
                                event_type: "waiting_input".to_string(),
                                session_id: session.id.clone(),
                                working_directory: session.working_directory.clone(),
                                timestamp: chrono::Local::now().to_rfc3339(),
                            };

                            // 通过 Tauri 事件系统发送
                            if let Err(e) = app_handle.emit("hook_event", &event) {
                                eprintln!("发送钩子事件失败: {}", e);
                            }
                        }
                        current_waiting.insert(session.id.clone());
                    }
                }

                // 检测 session 结束（之前在运行，现在不在列表中）
                for old_session_id in &last_waiting_sessions {
                    if !current_waiting.contains(old_session_id) {
                        let event = HookEvent {
                            event_type: "session_end".to_string(),
                            session_id: old_session_id.clone(),
                            working_directory: String::new(),
                            timestamp: chrono::Local::now().to_rfc3339(),
                        };

                        if let Err(e) = app_handle.emit("hook_event", &event) {
                            eprintln!("发送钩子事件失败: {}", e);
                        }
                    }
                }

                last_waiting_sessions = current_waiting;
            }

            // 每 2 秒检查一次
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

/// 手动触发钩子事件（用于测试）
pub fn trigger_hook_event(app_handle: &tauri::AppHandle, event: HookEvent) -> Result<(), String> {
    app_handle.emit("hook_event", &event)
        .map_err(|e| format!("发送事件失败: {}", e))
}