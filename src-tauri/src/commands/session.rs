use crate::utils::claude_data::{
    get_all_sessions, get_session_conversation, delete_session,
    ClaudeSession, Conversation,
};
use crate::utils::running_sessions::{
    init_running_sessions,
    get_running_sessions,
    start_polling,
    stop_polling,
    RunningSession,
    RUNNING_SESSIONS,
    refresh_git_info_background,
};
use crate::db::sessions_meta::get_session_names;
use tracing::{info, error};

/// 初始化运行中 session 列表（应用启动时调用）
#[tauri::command]
pub fn init_running() -> Result<Vec<RunningSession>, String> {
    info!("[init_running] 开始初始化运行中 session 列表");
    let result = init_running_sessions();
    match result {
        Ok(sessions) => {
            info!("[init_running] 完成，返回 {} 个 running session", sessions.len());
            Ok(sessions)
        }
        Err(e) => {
            error!("[init_running] 失败: {}", e);
            Err(e)
        }
    }
}

/// 获取运行中 session 列表
#[tauri::command]
pub fn list_running() -> Result<Vec<RunningSession>, String> {
    info!("[list_running] 开始获取运行中 session 列表");
    let sessions = get_running_sessions();

    // 获取所有 running session 的自定义名称
    let session_ids: Vec<String> = sessions.iter().map(|s| s.session_id.clone()).collect();
    match get_session_names(&session_ids) {
        Ok(custom_names) => {
            // 合并 custom_name
            let mut result = sessions;
            for session in &mut result {
                for (id, name) in &custom_names {
                    if session.session_id == *id {
                        session.custom_name = name.clone();
                        break;
                    }
                }
            }
            info!("[list_running] 完成，返回 {} 个 session（已合并 {} 个自定义名称）",
                  result.len(), custom_names.iter().filter(|(_, n)| n.is_some()).count());
            Ok(result)
        }
        Err(e) => {
            error!("[list_running] 获取自定义名称失败: {}", e);
            // 即使获取自定义名称失败，仍然返回 session 列表（custom_name 为 None）
            info!("[list_running] 完成，返回 {} 个 session（无自定义名称）", sessions.len());
            Ok(sessions)
        }
    }
}

/// 手动刷新所有运行中 session 的 git 信息（后台非阻塞，force 采集）。
#[tauri::command]
pub fn refresh_git_info_all(app_handle: tauri::AppHandle) -> Result<(), String> {
    info!("[refresh_git_info_all] 开始：对所有运行中 session 触发 git 信息采集");
    let pids: Vec<u32> = {
        let sessions = RUNNING_SESSIONS.lock().unwrap();
        sessions.keys().cloned().collect()
    };
    let count = pids.len();
    for pid in pids {
        refresh_git_info_background(pid, app_handle.clone(), true);
    }
    info!("[refresh_git_info_all] 已派发 {} 个 session 的采集任务", count);
    Ok(())
}

/// 启动定时轮询
#[tauri::command]
pub fn start_polling_cmd(app: tauri::AppHandle) -> Result<(), String> {
    info!("[start_polling_cmd] 开始启动定时轮询");
    start_polling(app);
    info!("[start_polling_cmd] 完成");
    Ok(())
}

/// 停止定时轮询
#[tauri::command]
pub fn stop_polling_cmd() -> Result<(), String> {
    info!("[stop_polling_cmd] 开始停止定时轮询");
    stop_polling();
    info!("[stop_polling_cmd] 完成");
    Ok(())
}

/// 获取指定 session 的对话内容
#[tauri::command]
pub fn get_conversation(session_id: String) -> Result<Conversation, String> {
    info!("[get_conversation] 开始获取对话，session_id: {}", session_id);
    let result = get_session_conversation(&session_id);
    match result {
        Ok(conversation) => {
            info!("[get_conversation] 完成，消息数: {}", conversation.total_messages);
            Ok(conversation)
        }
        Err(e) => {
            error!("[get_conversation] 失败: {}", e);
            Err(e)
        }
    }
}

/// 刷新 session 列表
#[tauri::command]
pub fn refresh_sessions() -> Result<Vec<ClaudeSession>, String> {
    info!("[refresh_sessions] 开始刷新 session 列表");
    let result = get_all_sessions();
    match result {
        Ok(sessions) => {
            info!("[refresh_sessions] 完成，返回 {} 个 session", sessions.len());
            Ok(sessions)
        }
        Err(e) => {
            error!("[refresh_sessions] 失败: {}", e);
            Err(e)
        }
    }
}

/// 删除 session
#[tauri::command]
pub fn delete_session_cmd(session_id: String) -> Result<(), String> {
    info!("[delete_session_cmd] 开始删除 session: {}", session_id);
    let result = delete_session(&session_id);
    match result {
        Ok(_) => {
            info!("[delete_session_cmd] 完成");
            Ok(())
        }
        Err(e) => {
            error!("[delete_session_cmd] 失败: {}", e);
            Err(e)
        }
    }
}

/// 启动新的 Claude Code session
#[tauri::command]
pub async fn start_new_session(
    working_directory: String,
    name: Option<String>,
    terminal_type: String,
) -> Result<String, String> {
    info!("[start_new_session] 开始启动新 session，工作目录: {}, 名称: {:?}, 终端: {}",
          working_directory, name, terminal_type);

    #[cfg(target_os = "windows")]
    {
        use crate::utils::launch::{launch_session, LaunchMode, LaunchRequest, LaunchSettings};

        let request = LaunchRequest {
            working_directory: working_directory.clone(),
            mode: LaunchMode::New { name: name.clone() },
            settings: LaunchSettings::legacy_default(&terminal_type),
        };

        launch_session(&request)
            .map_err(|e| {
                error!("[start_new_session] 启动失败: {}", e);
                e
            })?;
        info!("[start_new_session] 终端启动成功（独立进程）");
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        debug!("[start_new_session] macOS 平台，使用 open");
        Command::new("open")
            .args(["-a", "Terminal", &working_directory])
            .spawn()
            .map_err(|e| {
                error!("[start_new_session] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;
        info!("[start_new_session] 终端启动成功");
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        debug!("[start_new_session] Linux 平台，使用 gnome-terminal");
        Command::new("gnome-terminal")
            .args(["--working-directory", &working_directory, "-e", "claude --permission-mode bypassPermissions"])
            .spawn()
            .map_err(|e| {
                error!("[start_new_session] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;
        info!("[start_new_session] 终端启动成功");
    }

    let message = if let Some(session_name) = name {
        info!("[start_new_session] 完成，已启动 Claude Code (名称: {})", session_name);
        format!("已在 {} 启动 Claude Code (名称: {})", working_directory, session_name)
    } else {
        info!("[start_new_session] 完成，已启动 Claude Code");
        format!("已在 {} 启动 Claude Code", working_directory)
    };
    Ok(message)
}

/// 启动 sessions 目录监听服务
#[tauri::command]
pub fn start_sessions_watcher(app: tauri::AppHandle) -> Result<(), String> {
    info!("[start_sessions_watcher] 开始启动 sessions 监听服务");
    let result = crate::utils::sessions_watcher::start_sessions_watcher(app);
    match result {
        Ok(_) => {
            info!("[start_sessions_watcher] 完成");
            Ok(())
        }
        Err(e) => {
            error!("[start_sessions_watcher] 失败: {}", e);
            Err(e)
        }
    }
}

/// 停止 sessions 目录监听服务
#[tauri::command]
pub fn stop_sessions_watcher() -> Result<(), String> {
    info!("[stop_sessions_watcher] 开始停止 sessions 监听服务");
    crate::utils::sessions_watcher::stop_sessions_watcher();
    info!("[stop_sessions_watcher] 完成");
    Ok(())
}

/// 兼容旧命令：启动监听服务（调用 sessions_watcher）
#[tauri::command]
pub fn start_hooks(app: tauri::AppHandle) -> Result<(), String> {
    info!("[start_hooks] 兼容命令，调用 start_sessions_watcher");
    crate::utils::sessions_watcher::start_sessions_watcher(app)
}

/// 兼容旧命令：停止监听服务（调用 sessions_watcher）
#[tauri::command]
pub fn stop_hooks() -> Result<(), String> {
    info!("[stop_hooks] 兼容命令，调用 stop_sessions_watcher");
    crate::utils::sessions_watcher::stop_sessions_watcher();
    Ok(())
}