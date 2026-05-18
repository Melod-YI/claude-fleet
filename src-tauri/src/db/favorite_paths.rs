// src-tauri/src/db/favorite_paths.rs
// 常用路径管理

use rusqlite::Result;

/// 记录路径使用
pub fn record_path_usage(_path: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}

/// 移除常用路径
pub fn remove_favorite_path(_path: &str) -> Result<()> {
    // TODO: 实现
    Ok(())
}

/// 获取排序后的常用路径列表
pub fn get_sorted_favorite_paths() -> Result<Vec<String>> {
    // TODO: 实现
    Ok(vec![])
}