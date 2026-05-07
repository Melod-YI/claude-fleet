use crate::utils::session_types::{SessionMeta, SessionMessage};
use crate::utils::claude_session::{scan_sessions, get_session_messages, delete_session};
use tracing::info;

/// List all sessions - optimized version for management tab
#[tauri::command]
pub fn list_sessions_optimized() -> Result<Vec<SessionMeta>, String> {
    info!("[list_sessions_optimized] Scanning sessions");
    let sessions = scan_sessions();
    info!("[list_sessions_optimized] Found {} sessions", sessions.len());
    Ok(sessions)
}

/// Get messages for a specific session - optimized version
#[tauri::command]
pub fn get_session_messages_optimized(session_id: String) -> Result<Vec<SessionMessage>, String> {
    info!("[get_session_messages_optimized] Loading messages for {}", session_id);
    get_session_messages(&session_id)
}

/// Delete a session - optimized version
#[tauri::command]
pub fn delete_session_optimized(session_id: String) -> Result<bool, String> {
    info!("[delete_session_optimized] Deleting {}", session_id);
    delete_session(&session_id)
}