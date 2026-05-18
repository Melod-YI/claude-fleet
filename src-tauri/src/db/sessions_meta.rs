// src-tauri/src/db/sessions_meta.rs
// Session 自定义名称 CRUD 操作

use rusqlite::Result;

/// 设置 session 自定义名称
pub fn set_session_name(_session_id: &str, _name: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}

/// 获取 session 自定义名称
pub fn get_session_name(_session_id: &str) -> Result<Option<String>> {
    // TODO: 实现
    Ok(None)
}

/// 删除 session 自定义名称
pub fn delete_session_name(_session_id: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}