mod utils;
mod commands;

use commands::session::{
    list_sessions,
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_hooks,
    stop_hooks,
    receive_hook_event,
    send_notification,
    delete_session_cmd,
};
use commands::terminal::{jump_to_terminal, resume_in_terminal};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_conversation,
            refresh_sessions,
            start_new_session,
            start_hooks,
            stop_hooks,
            receive_hook_event,
            send_notification,
            jump_to_terminal,
            resume_in_terminal,
            delete_session_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}