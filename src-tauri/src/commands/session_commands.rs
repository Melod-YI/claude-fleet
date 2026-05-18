use crate::utils::session_types::{SessionMeta, SessionMessage};
use crate::utils::claude_session::{scan_sessions, get_session_messages, delete_session};
use crate::db::sessions_meta::get_session_names;
use tracing::info;

/// List all sessions - optimized version for management tab
#[tauri::command]
pub fn list_sessions_optimized() -> Result<Vec<SessionMeta>, String> {
    info!("[list_sessions_optimized] Scanning sessions");
    let sessions = scan_sessions();

    // 获取所有 session 的自定义名称
    let session_ids: Vec<String> = sessions.iter().map(|s| s.session_id.clone()).collect();
    let custom_names = get_session_names(&session_ids)
        .map_err(|e| format!("获取自定义名称失败: {}", e))?;

    // 合并 custom_name 到 session
    let mut result = sessions;
    for session in &mut result {
        for (id, name) in &custom_names {
            if session.session_id == *id {
                session.custom_name = name.clone();
                break;
            }
        }
    }

    info!("[list_sessions_optimized] Found {} sessions", result.len());
    Ok(result)
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