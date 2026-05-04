mod utils;
mod commands;

use commands::session::{
    list_sessions,
    init_running,
    list_running,
    start_polling_cmd,
    stop_polling_cmd,
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_hooks,
    stop_hooks,
    receive_hook_event,
    send_notification,
    delete_session_cmd,
};
use commands::terminal::{jump_to_terminal, jump_to_terminal_by_pid, smart_jump_to_terminal, resume_in_terminal};
use tracing::{info, error};
use std::time::Instant;

/// 应用启动初始化
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    info!("[setup] 应用启动初始化开始");

    // 初始化日志系统
    info!("[setup] 初始化日志系统");
    utils::logger::init_logging();
    info!("[setup] 日志系统初始化完成，日志目录: {}", utils::logger::get_log_dir().display());

    let app_handle = app.handle();

    // 初始化运行中 session 列表
    info!("[setup] 步骤1: 初始化运行中 session 列表");
    let init_start = Instant::now();
    if let Err(e) = init_running() {
        error!("[setup] 初始化运行中 session 列表失败: {}", e);
    } else {
        let elapsed = init_start.elapsed();
        info!("[setup] 运行中 session 列表初始化成功，耗时: {}ms", elapsed.as_millis());
    }

    // 启动 hook 监听
    info!("[setup] 步骤2: 启动 hook 监听服务");
    let hook_start = Instant::now();
    if let Err(e) = start_hooks(app_handle.clone()) {
        error!("[setup] 启动 hook 监听失败: {}", e);
    } else {
        let elapsed = hook_start.elapsed();
        info!("[setup] hook 监听服务启动成功，耗时: {}ms", elapsed.as_millis());
    }

    // 启动定时轮询
    info!("[setup] 步骤3: 启动定时轮询服务");
    let poll_start = Instant::now();
    if let Err(e) = start_polling_cmd(app_handle.clone()) {
        error!("[setup] 启动定时轮询失败: {}", e);
    } else {
        let elapsed = poll_start.elapsed();
        info!("[setup] 定时轮询服务启动成功，耗时: {}ms", elapsed.as_millis());
    }

    let elapsed = start.elapsed();
    info!("[setup] 应用启动初始化完成，总耗时: {}ms", elapsed.as_millis());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            init_running,
            list_running,
            start_polling_cmd,
            stop_polling_cmd,
            get_conversation,
            refresh_sessions,
            start_new_session,
            start_hooks,
            stop_hooks,
            receive_hook_event,
            send_notification,
            jump_to_terminal,
            jump_to_terminal_by_pid,
            smart_jump_to_terminal,
            resume_in_terminal,
            delete_session_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}