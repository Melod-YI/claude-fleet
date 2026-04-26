use crate::utils::claude_data::{
    get_all_sessions, get_session_conversation, ClaudeSession, Conversation,
};

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