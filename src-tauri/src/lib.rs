mod utils;
mod commands;
mod db;

use commands::session::{
    init_running,
    list_running,
    start_polling_cmd,
    stop_polling_cmd,
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_sessions_watcher,
    stop_sessions_watcher,
    start_hooks,
    stop_hooks,
    delete_session_cmd,
};
use commands::session_commands::{
    list_sessions_optimized,
    get_session_messages_optimized,
    delete_session_optimized,
};
use commands::terminal::{jump_to_terminal, jump_to_terminal_by_pid, smart_jump_to_terminal, resume_in_terminal, launch_session, open_directory, open_in_vscode};
use commands::sound::{get_available_sounds, get_sound_data};
use commands::worktree::{create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd, delete_worktree_cmd, preflight_delete_worktree_cmd, count_worktrees_cmd};
// 数据库命令
use db::sessions_meta::{set_session_name_cmd, get_session_name_cmd, delete_session_name_cmd};
use db::favorites::{add_favorite_cmd, remove_favorite_cmd, is_favorite_cmd, get_all_favorites_cmd};
use db::favorite_paths::{record_path_usage_cmd, remove_favorite_path_cmd, get_sorted_favorite_paths_cmd, toggle_pin_path_cmd};
use db::settings::{get_setting_cmd, set_setting_cmd, get_all_settings_cmd};
use db::migration::needs_migration_cmd;
use db::tracked_repos::{add_tracked_repo_cmd, remove_tracked_repo_cmd, list_tracked_repos_cmd};
use tracing::{info, error};
use std::time::Instant;
use tauri::Emitter;

/// 应用启动初始化
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    info!("[setup] 应用启动初始化开始");

    // 初始化日志系统
    info!("[setup] 初始化日志系统");
    utils::logger::init_logging();
    info!("[setup] 日志系统初始化完成，日志目录: {}", utils::logger::get_log_dir().display());

    // 初始化数据库表（确保所有表存在）
    info!("[setup] 步骤0: 初始化数据库表");
    if let Err(e) = db::schema::init_tables() {
        error!("[setup] 初始化数据库表失败: {}", e);
    } else {
        info!("[setup] 数据库表初始化成功");
    }

    let app_handle = app.handle();

    // 后台初始化运行中 session 列表（不阻塞 WebView 加载）
    // 完成后通过事件通知前端
    info!("[setup] 步骤1: 后台启动运行中 session 初始化");
    {
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            let init_start = Instant::now();
            match utils::running_sessions::init_running_sessions() {
                Ok(sessions) => {
                    let elapsed = init_start.elapsed();
                    info!("[setup] 后台 session 初始化完成，{} 个 session，耗时: {}ms", sessions.len(), elapsed.as_millis());
                    if let Err(e) = handle.emit("running_sessions_changed", &sessions) {
                        error!("[setup] 发送 running_sessions_changed 事件失败: {}", e);
                    }
                }
                Err(e) => {
                    error!("[setup] 后台初始化运行中 session 列表失败: {}", e);
                }
            }
        });
    }

    // 启动 sessions 目录监听服务
    info!("[setup] 步骤2: 启动 sessions 目录监听服务");
    let watcher_start = Instant::now();
    if let Err(e) = start_sessions_watcher(app_handle.clone()) {
        error!("[setup] 启动 sessions 监听失败: {}", e);
    } else {
        let elapsed = watcher_start.elapsed();
        info!("[setup] sessions 监听服务启动成功，耗时: {}ms", elapsed.as_millis());
    }

    // 启动定时轮询（检测意外退出）
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
    // 设置 Ctrl+C 处理，优雅退出
    ctrlc::set_handler(|| {
        tracing::info!("[exit] 收到 Ctrl+C 信号，应用退出");
        std::process::exit(0);
    }).expect("Failed to set Ctrl+C handler");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            // New optimized session commands for management tab
            list_sessions_optimized,
            get_session_messages_optimized,
            delete_session_optimized,
            // Running session commands (keep for Running Tab)
            init_running,
            list_running,
            start_polling_cmd,
            stop_polling_cmd,
            // Legacy commands (keep for compatibility)
            get_conversation,
            refresh_sessions,
            start_new_session,
            start_sessions_watcher,
            stop_sessions_watcher,
            start_hooks,
            stop_hooks,
            delete_session_cmd,
            // Terminal commands
            jump_to_terminal,
            jump_to_terminal_by_pid,
            smart_jump_to_terminal,
            resume_in_terminal,
            launch_session,
            open_directory,
            open_in_vscode,
            // Sound commands
            get_available_sounds,
            get_sound_data,
            // 数据库命令
            set_session_name_cmd,
            get_session_name_cmd,
            delete_session_name_cmd,
            add_favorite_cmd,
            remove_favorite_cmd,
            is_favorite_cmd,
            get_all_favorites_cmd,
            record_path_usage_cmd,
            remove_favorite_path_cmd,
            get_sorted_favorite_paths_cmd,
            toggle_pin_path_cmd,
            get_setting_cmd,
            set_setting_cmd,
            get_all_settings_cmd,
            needs_migration_cmd,
            // Worktree commands
            create_worktree_cmd,
            list_worktrees_cmd,
            get_repo_info_cmd,
            delete_worktree_cmd,
            preflight_delete_worktree_cmd,
            count_worktrees_cmd,
            // Tracked repos commands
            add_tracked_repo_cmd,
            remove_tracked_repo_cmd,
            list_tracked_repos_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}