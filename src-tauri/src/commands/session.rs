use crate::utils::claude_data::{
    get_all_sessions, get_session_conversation, delete_session, ClaudeSession, Conversation,
};
use crate::utils::hooks::{start_hook_server, stop_hook_server, handle_hook_event, HookEvent};

/// 获取所有 session 列表
#[tauri::command]
pub fn list_sessions() -> Result<Vec<ClaudeSession>, String> {
    get_all_sessions()
}

/// 获取指定 session 的对话内容
#[tauri::command]
pub fn get_conversation(session_id: String) -> Result<Conversation, String> {
    get_session_conversation(&session_id)
}

/// 刷新 session 列表
#[tauri::command]
pub fn refresh_sessions() -> Result<Vec<ClaudeSession>, String> {
    get_all_sessions()
}

/// 删除 session
#[tauri::command]
pub fn delete_session_cmd(session_id: String) -> Result<(), String> {
    delete_session(&session_id)
}

/// 启动新的 Claude Code session
#[tauri::command]
pub async fn start_new_session(
    app: tauri::AppHandle,
    working_directory: String,
    name: Option<String>,
) -> Result<String, String> {
    // 使用 shell plugin 启动 Windows Terminal
    use tauri_plugin_shell::ShellExt;

    let terminal_cmd = if cfg!(target_os = "windows") {
        // Windows: 使用 wt (Windows Terminal)
        format!("wt -d \"{}\" claude", working_directory)
    } else if cfg!(target_os = "macos") {
        // macOS: 使用 open 命令打开 Terminal
        format!("open -a Terminal \"{}\"", working_directory)
    } else {
        // Linux: 使用 gnome-terminal
        format!("gnome-terminal --working-directory=\"{}\" -e claude", working_directory)
    };

    // 执行命令
    let shell = app.shell();
    let result = shell
        .command("sh")
        .args(["-c", &terminal_cmd])
        .output()
        .await;

    match result {
        Ok(_) => {
            let message = if let Some(session_name) = name {
                format!("已在 {} 启动 Claude Code (名称: {})", working_directory, session_name)
            } else {
                format!("已在 {} 启动 Claude Code", working_directory)
            };
            Ok(message)
        }
        Err(e) => Err(format!("启动失败: {}", e)),
    }
}

/// 启动钩子服务
#[tauri::command]
pub fn start_hooks(app: tauri::AppHandle) -> Result<(), String> {
    start_hook_server(app)
}

/// 停止钩子服务
#[tauri::command]
pub fn stop_hooks() -> Result<(), String> {
    stop_hook_server();
    Ok(())
}

/// 接收钩子事件
#[tauri::command]
pub fn receive_hook_event(event: HookEvent) -> Result<(), String> {
    handle_hook_event(event)
}

/// 发送桌面通知
#[tauri::command]
pub fn send_notification(title: String, body: String) -> Result<(), String> {
    // 这里可以集成系统通知
    // 目前只是打印日志，前端会使用 Web Notifications API
    println!("通知: {} - {}", title, body);
    Ok(())
}