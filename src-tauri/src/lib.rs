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

/// 应用启动初始化
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();

    // 初始化运行中 session 列表
    init_running().ok();

    // 启动 hook 监听
    start_hooks(app_handle.clone()).ok();

    // 启动定时轮询
    start_polling_cmd(app_handle.clone()).ok();

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