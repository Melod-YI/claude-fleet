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
};
use tracing::{info, debug, error};

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
    use std::process::Command;
    use crate::utils::window_manager::get_terminal_config_for_new;
    info!("[start_new_session] 开始启动新 session，工作目录: {}, 名称: {:?}, 终端: {}",
          working_directory, name, terminal_type);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;

        // 获取终端配置（新建 session）
        let config = match get_terminal_config_for_new(&terminal_type) {
            Some(c) => c,
            None => {
                error!("[start_new_session] 不支持的终端类型: {}", terminal_type);
                return Err(format!("不支持的终端类型: {}", terminal_type));
            }
        };

        info!("[start_new_session] 终端配置: {} {}", config.command, config.args.join(" "));

        // 替换参数中的变量
        let mut args: Vec<String> = config.args.iter().map(|arg| {
            arg.replace("{cwd}", &working_directory)
        }).collect();

        // 如果有名称，添加 --name 参数到末尾
        if let Some(ref session_name) = name {
            let trimmed_name = session_name.trim();
            if !trimmed_name.is_empty() {
                args.push("--name".to_string());
                args.push(trimmed_name.to_string());
            }
        }

        let mut cmd = Command::new(config.command);
        cmd.args(&args);

        // wezterm 是 GUI 程序需要 DETACHED_PROCESS
        // cmd/powershell 通过 start 命令启动独立进程，父 cmd.exe 立即退出
        if terminal_type == "wezterm" {
            cmd.creation_flags(DETACHED_PROCESS);
        }

        cmd.spawn()
            .map_err(|e| {
                error!("[start_new_session] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;
        info!("[start_new_session] 终端启动成功（独立进程）");
    }

    #[cfg(target_os = "macos")]
    {
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

/// 发送桌面通知
#[tauri::command]
pub fn send_notification(title: String, body: String) -> Result<(), String> {
    info!("[send_notification] 发送通知: 标题=\"{}\", 内容=\"{}\"", title, body);
    // 这里可以集成系统通知
    // 目前只是打印日志，前端会使用 Web Notifications API
    debug!("[send_notification] 通知已记录（前端将使用 Web Notifications API）");
    Ok(())
}