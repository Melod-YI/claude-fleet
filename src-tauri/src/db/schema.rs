// src-tauri/src/db/schema.rs

use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tracing::{error, info};

/// 获取数据库文件路径 ~/.claude-fleet/data/claude-fleet.db
pub fn get_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("无法获取用户目录")
        .join(".claude-fleet")
        .join("data")
        .join("claude-fleet.db")
}

/// 应用生命周期内常驻的"看门狗"连接（issue#3 根因修复，见 establish_keepalive）。
static KEEPALIVE: OnceLock<Mutex<Connection>> = OnceLock::new();

/// 打开一个到指定路径的连接并应用并发安全 pragma。
fn build_connection(db_path: &Path) -> Result<Connection> {
    // 确保 data 目录存在
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|_e| rusqlite::Error::InvalidPath(parent.to_path_buf()))?;
            info!("[build_connection] 创建数据目录: {}", parent.display());
        }
    }

    let conn = Connection::open(db_path)?;
    apply_concurrency_pragmas(&conn)?;
    Ok(conn)
}

/// 获取数据库连接（每次新建）。由 init_tables 建立的 keepalive 常驻连接保证
/// 此连接关闭时不是"最后一个连接"，不会触发 close-checkpoint 写主库页。
pub fn get_connection() -> Result<Connection> {
    build_connection(&get_db_path())
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

/// 记录 DB 错误到日志并返回前端可读字符串。issue#3 次生缺陷修复：
/// 原先 `.map_err(|e| format!("..: {}", e))` 把 rusqlite 错误静默吞成字符串，
/// 事后日志里 grep 不到 `malformed` 等关键字，无法定位损坏。统一走此 helper
/// 确保 DB 错误进 `error!()` 日志。用户可见字符串与原先 `format!("context: {e}")` 一致。
pub fn log_db_err<E: std::fmt::Display>(context: &str, e: E) -> String {
    error!("[db] {context}: {e}");
    format!("{context}: {e}")
}

/// 建立应用生命周期内常驻的"看门狗"连接（issue#3 根因修复）。
///
/// 原先每次 DB 调用都 open→用→drop，每次连接关闭若为"最后一个连接"会触发
/// close-checkpoint 把 WAL 帧写回主库页；若进程恰在此写主库页期间被强杀
/// （关窗/注销/任务管理器/worktree 创建期间 spawn git 耗时数秒、用户中途关窗），
/// 主库数据页被写半截 → 永久页级损坏。`worktrees` 表写最密集且常伴随耗时 git
/// 子进程，故损坏集中在该表。
///
/// 保持一条连接常驻 → 任何业务连接关闭时都不是"最后一个连接" → 不再触发
/// close-checkpoint；WAL 仍由 SQLite 默认 `wal_autocheckpoint`（1000 页阈值，
/// 事务提交后安全合并，抗强杀）正常管理。从根上消除"每调用一次就写一次主库"
/// 的高频损坏窗口。
///
/// 幂等：重复调用是 no-op。`init_tables` 末尾调用一次即可，无需在 setup 单独触发。
pub fn establish_keepalive(conn: Connection) {
    if KEEPALIVE.set(Mutex::new(conn)).is_err() {
        info!("[establish_keepalive] 已存在，跳过");
    } else {
        info!("[establish_keepalive] 常驻连接已建立，抑制后续业务连接的 close-checkpoint");
    }
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

    // issue#3 根因修复：将此连接常驻为 keepalive，使后续每次业务连接关闭时
    // 都不是"最后一个连接"，不再触发 close-checkpoint 写主库页（损坏来源）。
    establish_keepalive(conn);

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

    #[test]
    fn build_connection_applies_wal_and_is_usable() {
        let dir = unique_dir("build");
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join("t.db");

        let conn = build_connection(&db).unwrap();
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode.to_lowercase(), "wal", "build_connection 应启用 WAL");
        conn.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(42);").unwrap();
        let v: i64 = conn.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 42, "build 出的连接应可建表/读写");
        drop(conn);
        let _ = fs::remove_dir_all(&dir);
    }

    /// 回归守卫（issue#3 根因修复依赖的不变式）。
    ///
    /// prod 中 `init_tables` 末尾调用 `establish_keepalive`，且该连接在 init_tables
    /// 期间执行了 `CREATE TABLE`（写过 → 持有 WAL read-mark）。持有 read-mark 的
    /// 常驻连接会使后续业务连接关闭时**不触发 close-checkpoint 清空 -wal**，从而
    /// 消除"每次业务连接关闭都把 WAL 帧写回主库页"的高频损坏窗口（进程被强杀于
    /// 该写主库页期间即页级损坏，issue#3 的 worktrees 表损坏即落于此）。
    ///
    /// 注：真实的"强杀于写主库页期间"是非确定时序，无法在单测中确定复现；本测试
    /// 锁定 keepalive 对 close-checkpoint 的抑制行为本身（含"无 keepalive 则清空"
    /// 的危险基线对照），防止未来误删 `establish_keepalive` 而无人察觉。
    #[test]
    fn keepalive_with_readmark_suppresses_close_checkpoint() {
        let dir = unique_dir("keep");
        fs::create_dir_all(&dir).unwrap();
        let db = dir.join("t.db");
        let wal = db.with_extension("db-wal");
        let sz = || fs::metadata(&wal).map(|m| m.len()).unwrap_or(0);

        // 危险基线：无 keepalive，写连接关闭（last conn）→ close-checkpoint 清空 -wal
        {
            let w = build_connection(&db).unwrap();
            w.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(1);").unwrap();
            assert!(sz() > 0, "写入后 -wal 应有帧");
            drop(w);
            assert_eq!(sz(), 0, "无 keepalive 时，写连接关闭应 close-checkpoint 清空 -wal（危险基线）");
            // 清理本 db，供下一场景重建
            for ext in ["", "-wal", "-shm"] {
                let _ = std::fs::remove_file(dir.join(format!("t.db{ext}")));
            }
        }

        // 修复场景：keepalive 写过（镜像 init_tables 建表），写连接关闭不清空 -wal
        {
            let keep = build_connection(&db).unwrap();
            keep.execute_batch("CREATE TABLE t(x);").unwrap(); // 镜像 init_tables 建表
            let w = build_connection(&db).unwrap();
            w.execute_batch("INSERT INTO t VALUES(1);").unwrap();
            let before = sz();
            drop(w);
            assert!(
                sz() == before && before > 0,
                "keepalive（写过，镜像 init_tables）在场时，写连接关闭不应 close-checkpoint 清空 -wal"
            );

            // 数据经 WAL 仍可读
            let r = build_connection(&db).unwrap();
            let v: i64 = r.query_row("SELECT x FROM t", [], |row| row.get(0)).unwrap();
            assert_eq!(v, 1);
            drop(r);

            // 释放 keepalive（last conn）→ 触发 close-checkpoint → -wal 清空
            drop(keep);
            assert_eq!(sz(), 0, "keepalive 释放（last conn）应触发 close-checkpoint 清空 -wal");
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_db_err_formats_and_contains_error_text() {
        // tracing 输出不在单测断言范围；仅验证返回字符串格式
        let s = log_db_err("数据库连接失败", "disk image is malformed");
        assert!(s.contains("数据库连接失败"));
        assert!(s.contains("malformed"));
    }
}



