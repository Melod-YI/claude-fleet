// src-tauri/src/db/sessions_meta.rs
// Session 自定义名称 CRUD 操作

use rusqlite::Result;
use tracing::{info, error};
use crate::db::schema::get_connection;

/// 设置 session 自定义名称
pub fn set_session_name(session_id: &str, name: &str) -> Result<()> {
    info!("[set_session_name] 设置名称: session_id={}, name={}", session_id, name);
    let conn = get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    conn.execute(
        "INSERT OR REPLACE INTO sessions_meta (session_id, custom_name, created_at, updated_at)
         VALUES (?1, ?2, COALESCE((SELECT created_at FROM sessions_meta WHERE session_id = ?1), ?3), ?3)",
        [session_id, name, &now.to_string()],
    )?;

    info!("[set_session_name] 成功设置名称");
    Ok(())
}

/// 获取 session 自定义名称
pub fn get_session_name(session_id: &str) -> Result<Option<String>> {
    let conn = get_connection()?;

    let result = conn.query_row(
        "SELECT custom_name FROM sessions_meta WHERE session_id = ?1",
        [session_id],
        |row| row.get::<_, Option<String>>(0),
    );

    match result {
        Ok(name) => Ok(name),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => {
            error!("[get_session_name] 查询失败: {}", e);
            Err(e)
        }
    }
}

/// 删除 session 自定义名称
pub fn delete_session_name(session_id: &str) -> Result<()> {
    info!("[delete_session_name] 删除名称: session_id={}", session_id);
    let conn = get_connection()?;

    conn.execute(
        "DELETE FROM sessions_meta WHERE session_id = ?1",
        [session_id],
    )?;

    Ok(())
}

/// 批量获取多个 session 的自定义名称
pub fn get_session_names(session_ids: &[String]) -> Result<Vec<(String, Option<String>)>> {
    if session_ids.is_empty() {
        return Ok(Vec::new());
    }

    let conn = get_connection()?;
    let mut results = Vec::new();

    for session_id in session_ids {
        let name = conn.query_row(
            "SELECT custom_name FROM sessions_meta WHERE session_id = ?1",
            [session_id],
            |row| row.get::<_, Option<String>>(0),
        ).ok().flatten();
        results.push((session_id.clone(), name));
    }

    Ok(results)
}

// Tauri 命令包装

#[tauri::command]
pub fn set_session_name_cmd(session_id: String, name: String) -> Result<(), String> {
    set_session_name(&session_id, &name).map_err(|e| format!("设置名称失败: {}", e))
}

#[tauri::command]
pub fn get_session_name_cmd(session_id: String) -> Result<Option<String>, String> {
    get_session_name(&session_id).map_err(|e| format!("获取名称失败: {}", e))
}

#[tauri::command]
pub fn delete_session_name_cmd(session_id: String) -> Result<(), String> {
    delete_session_name(&session_id).map_err(|e| format!("删除名称失败: {}", e))
}