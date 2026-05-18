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

/// 初始化数据库表结构
pub fn init_database() -> Result<()> {
    info!("[init_database] 开始初始化数据库");
    let conn = get_connection()?;

    // Session 自定义名称表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions_meta (
            session_id    TEXT PRIMARY KEY,
            custom_name   TEXT,
            created_at    INTEGER,
            updated_at    INTEGER
        )",
        [],
    )?;

    // 收藏列表表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS favorites (
            session_id    TEXT PRIMARY KEY,
            added_at      INTEGER
        )",
        [],
    )?;

    // 常用路径表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS favorite_paths (
            path          TEXT PRIMARY KEY,
            use_count     INTEGER DEFAULT 1,
            last_used_at  INTEGER
        )",
        [],
    )?;

    // 应用设置表（KV 存储）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key           TEXT PRIMARY KEY,
            value         TEXT
        )",
        [],
    )?;

    info!("[init_database] 数据库初始化完成，路径: {}", get_db_path().display());
    Ok(())
}