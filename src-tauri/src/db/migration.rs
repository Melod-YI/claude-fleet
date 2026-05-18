// src-tauri/src/db/migration.rs
// 从 localStorage 迁移数据

use tracing::{error, info};
use crate::db::favorites::get_all_favorites;
use crate::db::schema::init_database;

/// 执行 localStorage 到 SQLite 的迁移检查
/// 返回是否需要前端执行迁移（true 表示有数据需迁移）
pub fn migrate_from_localstorage() -> bool {
    info!("[migrate_from_localstorage] 开始检查是否需要迁移");

    // 1. 初始化数据库
    if let Err(e) = init_database() {
        error!("[migrate_from_localstorage] 数据库初始化失败: {}", e);
        return false;
    }

    // 2. 检查是否已有数据（如果有，说明已经迁移过了）
    let existing_favorites = get_all_favorites().unwrap_or_default();
    if !existing_favorites.is_empty() {
        info!(
            "[migrate_from_localstorage] 已有 {} 个收藏，跳过迁移",
            existing_favorites.len()
        );
        return false;
    }

    info!("[migrate_from_localstorage] 检测到需要迁移，返回 true");
    true
}

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