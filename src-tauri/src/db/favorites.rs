// src-tauri/src/db/favorites.rs
// 收藏列表 CRUD 操作

use rusqlite::Result;
use tracing::info;
use crate::db::schema::get_connection;

/// 添加收藏
pub fn add_favorite(session_id: &str) -> Result<()> {
    info!("[add_favorite] 添加收藏: session_id={}", session_id);
    let conn = get_connection()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    conn.execute(
        "INSERT OR IGNORE INTO favorites (session_id, added_at) VALUES (?1, ?2)",
        [session_id, &now.to_string()],
    )?;

    info!("[add_favorite] 成功添加收藏");
    Ok(())
}

/// 移除收藏
pub fn remove_favorite(session_id: &str) -> Result<()> {
    info!("[remove_favorite] 移除收藏: session_id={}", session_id);
    let conn = get_connection()?;

    conn.execute(
        "DELETE FROM favorites WHERE session_id = ?1",
        [session_id],
    )?;

    info!("[remove_favorite] 成功移除收藏");
    Ok(())
}

/// 检查是否已收藏
pub fn is_favorite(session_id: &str) -> Result<bool> {
    let conn = get_connection()?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM favorites WHERE session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// 获取所有收藏
pub fn get_all_favorites() -> Result<Vec<String>> {
    info!("[get_all_favorites] 获取所有收藏");
    let conn = get_connection()?;

    let mut stmt = conn.prepare("SELECT session_id FROM favorites ORDER BY added_at DESC")?;
    let session_ids = stmt.query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<String>>>()?;

    info!("[get_all_favorites] 共 {} 个收藏", session_ids.len());
    Ok(session_ids)
}