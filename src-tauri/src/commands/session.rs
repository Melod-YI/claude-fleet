use crate::utils::claude_data::{
    get_all_sessions, get_session_conversation, delete_session,
    ClaudeSession, Conversation,
};
use crate::utils::hooks::{start_hook_receiver, stop_hook_receiver, handle_hook_event};
use crate::utils::running_sessions::{
    init_running_sessions,
    get_running_sessions,
    start_polling,
    stop_polling,
    RunningSession,
    HookEvent,
};
use tracing::{info, debug, error};

/// 获取所有 session 列表（用于管理 Tab）
#[tauri::command]
pub fn list_sessions() -> Result<Vec<ClaudeSession>, String> {
    info!("[list_sessions] 开始获取 session 列表");
    let result = get_all_sessions();
    match result {
        Ok(sessions) => {
            info!("[list_sessions] 完成，返回 {} 个 session", sessions.len());
            Ok(sessions)
        }
        Err(e) => {
            error!("[list_sessions] 失败: {}", e);
            Err(e)
        }
    }
}

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
    info!("[list_running] 完成，返回 {} 个 session", sessions.len());
    Ok(sessions)
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

// 旧命令已废弃，使用 list_running 替代
// #[tauri::command]
// pub fn list_running_sessions() -> Result<Vec<ClaudeSession>, String> {
//     get_running_sessions_list()
// }

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
    app: tauri::AppHandle,
    working_directory: String,
    name: Option<String>,
) -> Result<String, String> {
    info!("[start_new_session] 开始启动新 session，工作目录: {}, 名称: {:?}",
          working_directory, name);

    // 使用 shell plugin 启动 Windows Terminal
    use tauri_plugin_shell::ShellExt;

    let terminal_cmd = if cfg!(target_os = "windows") {
        debug!("[start_new_session] Windows 平台，使用 wt");
        format!("wt -d \"{}\" claude", working_directory)
    } else if cfg!(target_os = "macos") {
        debug!("[start_new_session] macOS 平台，使用 open");
        format!("open -a Terminal \"{}\"", working_directory)
    } else {
        debug!("[start_new_session] Linux 平台，使用 gnome-terminal");
        format!("gnome-terminal --working-directory=\"{}\" -e claude", working_directory)
    };

    info!("[start_new_session] 终端命令: {}", terminal_cmd);

    // 执行命令
    let shell = app.shell();
    debug!("[start_new_session] 执行 shell 命令");
    let result = shell
        .command("sh")
        .args(["-c", &terminal_cmd])
        .output()
        .await;

    match result {
        Ok(output) => {
            debug!("[start_new_session] 命令执行完成，状态: {:?}", output.status);
            let message = if let Some(session_name) = name {
                info!("[start_new_session] 完成，已启动 Claude Code (名称: {})", session_name);
                format!("已在 {} 启动 Claude Code (名称: {})", working_directory, session_name)
            } else {
                info!("[start_new_session] 完成，已启动 Claude Code");
                format!("已在 {} 启动 Claude Code", working_directory)
            };
            Ok(message)
        }
        Err(e) => {
            error!("[start_new_session] 启动失败: {}", e);
            Err(format!("启动失败: {}", e))
        }
    }
}

/// 启动钩子服务（文件监听方式）
#[tauri::command]
pub fn start_hooks(app: tauri::AppHandle) -> Result<(), String> {
    info!("[start_hooks] 开始启动钩子服务");
    let result = start_hook_receiver(app);
    match result {
        Ok(_) => {
            info!("[start_hooks] 完成");
            Ok(())
        }
        Err(e) => {
            error!("[start_hooks] 失败: {}", e);
            Err(e)
        }
    }
}

/// 停止钩子服务
#[tauri::command]
pub fn stop_hooks() -> Result<(), String> {
    info!("[stop_hooks] 开始停止钩子服务");
    stop_hook_receiver();
    info!("[stop_hooks] 完成");
    Ok(())
}

/// 接收钩子事件
#[tauri::command]
pub fn receive_hook_event(event: HookEvent) -> Result<(), String> {
    info!("[receive_hook_event] 开始接收钩子事件: type={}, session_id={}",
          event.hook_event_name, event.session_id);
    let result = handle_hook_event(event);
    match result {
        Ok(_) => {
            info!("[receive_hook_event] 完成");
            Ok(())
        }
        Err(e) => {
            error!("[receive_hook_event] 失败: {}", e);
            Err(e)
        }
    }
}

/// 发送桌面通知
#[tauri::command]
pub fn send_notification(title: String, body: String) -> Result<(), String> {
    info!("[send_notification] 发送通知: 标题=\"{}\", 内容=\"{}\"", title, body);
    // 这里可以集成系统通知
    // 目前只是打印日志，前端会使用 Web Notifications API
    debug!("[send_notification] 通知已记录（前端将使用 Web Notifications API）");
    Ok(())
}