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

    let conn = Connection::open(&db_path)?;
    // 多进程并发安全配置（每连接设置，见 apply_concurrency_pragmas 注释）
    apply_concurrency_pragmas(&conn)?;
    Ok(conn)
}

/// 为连接应用多进程并发安全 pragma。
///
/// - `journal_mode=WAL`：写先追加到独立 -wal 文件，主库事务期不被改动，强杀只留
///   可丢弃的 -wal 残帧（下次打开自动恢复），天然抗 TerminateProcess 中断写。
///   WAL 持久化进 DB 头，重复设置幂等。
/// - `busy_timeout=5000`：并发写等待 5s 而非立即 SQLITE_BUSY（两实例同时写、
///   前端 Promise.all 并发写受益）。
/// - `synchronous=NORMAL(=1)`：WAL 下不损坏且更快；仅掉电可能丢最后一个事务，
///   不影响强杀场景（强杀不丢已提交事务）。
fn apply_concurrency_pragmas(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.pragma_update(None, "busy_timeout", 5000_i64)?;
    conn.pragma_update(None, "synchronous", 1_i64)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 生成唯一临时目录，避免并行/重复运行碰撞
    fn unique_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("cf_schema_{}_{}_{}", tag, std::process::id(), nanos))
    }

    #[test]
    fn apply_concurrency_pragmas_enables_wal_and_busy_timeout() {
        let dir = unique_dir("pragma");
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join("test.db");

        // 应用后同连接验证 per-connection 设置
        let conn = Connection::open(&db).unwrap();
        apply_concurrency_pragmas(&conn).unwrap();
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode.to_lowercase(), "wal", "journal_mode 应为 WAL");
        let bt: i64 = conn.query_row("PRAGMA busy_timeout", [], |r| r.get(0)).unwrap();
        assert_eq!(bt, 5000, "busy_timeout 应为 5000ms");
        let sync: i64 = conn.query_row("PRAGMA synchronous", [], |r| r.get(0)).unwrap();
        assert_eq!(sync, 1, "synchronous 应为 NORMAL(1)");
        drop(conn);

        // 重新打开：WAL 应持久化进 DB 头（per-conn 的 busy_timeout/synchronous 不持久，不校验）
        let conn2 = Connection::open(&db).unwrap();
        let mode2: String = conn2.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode2.to_lowercase(), "wal", "WAL 应持久化");
        let _ = fs::remove_dir_all(&dir);
    }
}



