mod utils;
mod commands;

use commands::session::{list_sessions, get_conversation, refresh_sessions};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            get_conversation,
            refresh_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}