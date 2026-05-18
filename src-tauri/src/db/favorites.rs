// src-tauri/src/db/favorites.rs
// 收藏列表 CRUD 操作

use rusqlite::Result;

/// 添加收藏
pub fn add_favorite(_session_id: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}

/// 移除收藏
pub fn remove_favorite(_session_id: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}

/// 检查是否已收藏
pub fn is_favorite(_session_id: &str) -> Result<bool> {
    // TODO: 实现
    Ok(false)
}

/// 获取所有收藏
pub fn get_all_favorites() -> Result<Vec<String>> {
    // TODO: 实现
    Ok(vec![])
}