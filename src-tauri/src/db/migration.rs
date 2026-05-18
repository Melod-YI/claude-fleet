// src-tauri/src/db/migration.rs
// 从 localStorage 迁移数据

use crate::db::favorites::get_all_favorites;

/// 检查是否需要迁移
pub fn needs_migration() -> bool {
    let existing_favorites = get_all_favorites().unwrap_or_default();
    existing_favorites.is_empty()
}

// Tauri 命令包装

#[tauri::command]
pub fn needs_migration_cmd() -> Result<bool, String> {
    Ok(needs_migration())
}