// src-tauri/src/db/schema.rs

use rusqlite::{Connection, Result};
use std::path::PathBuf;
use tracing::info;

/// 获取数据库文件路径 ~/.claude-fleet/data/claude-fleet.db
pub fn get_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("data")
        .join("claude-fleet.db")
}

/// 获取数据库连接
pub fn get_connection() -> Result<Connection> {
    let db_path = get_db_path();

    // 确保 data 目录存在
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|_e| rusqlite::Error::InvalidPath(parent.to_path_buf()))?;
            info!("[get_connection] 创建数据目录: {}", parent.display());
        }
    }

    Connection::open(&db_path)
}

/// 初始化数据库表（创建缺失的表）
pub fn init_tables() -> Result<()> {
    info!("[init_tables] 开始初始化数据库表");
    let conn = get_connection()?;

    // 使用 IF NOT EXISTS 确保只创建缺失的表，已存在的表不受影响
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS favorites (
            session_id TEXT PRIMARY KEY,
            added_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT
        );
        CREATE TABLE IF NOT EXISTS sessions_meta (
            session_id TEXT PRIMARY KEY,
            custom_name TEXT,
            created_at INTEGER,
            updated_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS favorite_paths (
            path TEXT PRIMARY KEY,
            use_count INTEGER,
            last_used_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS worktrees (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            branch TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            repo_name TEXT NOT NULL,
            repo_path TEXT NOT NULL,
            base_ref TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS tracked_repos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            added_at INTEGER NOT NULL
        );"
    )?;

    // 迁移：为 favorite_paths 表添加 pinned 和 pinned_at 列（如果不存在）
    let pinned_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('favorite_paths') WHERE name='pinned'",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if !pinned_exists {
        conn.execute("ALTER TABLE favorite_paths ADD COLUMN pinned INTEGER DEFAULT 0", [])?;
        conn.execute("ALTER TABLE favorite_paths ADD COLUMN pinned_at INTEGER DEFAULT NULL", [])?;
        info!("[init_tables] 添加 pinned 和 pinned_at 列");
    }

    info!("[init_tables] 数据库表初始化完成");
    Ok(())
}

