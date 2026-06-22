// src-tauri/src/db/tracked_repos.rs
// 跟踪仓库管理

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::db::schema::get_connection;

/// 跟踪的仓库记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedRepo {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub added_at: i64,
}

/// 添加跟踪仓库。path 有 UNIQUE 约束，重复插入会报错。
pub fn add_tracked_repo(conn: &Connection, path: &str, name: &str) -> Result<TrackedRepo> {
    tracing::debug!("[add_tracked_repo] 添加仓库: path={}, name={}", path, name);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    conn.execute(
        "INSERT INTO tracked_repos (path, name, added_at) VALUES (?1, ?2, ?3)",
        params![path, name, now],
    )?;

    let id = conn.last_insert_rowid();
    info!("[add_tracked_repo] 成功添加: id={}", id);

    Ok(TrackedRepo {
        id,
        path: path.to_string(),
        name: name.to_string(),
        added_at: now,
    })
}

/// 删除跟踪仓库
pub fn remove_tracked_repo(conn: &Connection, id: i64) -> Result<()> {
    tracing::debug!("[remove_tracked_repo] 删除仓库: id={}", id);
    let changes = conn.execute("DELETE FROM tracked_repos WHERE id = ?1", params![id])?;
    if changes == 0 {
        tracing::warn!("[remove_tracked_repo] 未找到 id={}", id);
    }
    info!("[remove_tracked_repo] 成功删除");
    Ok(())
}

/// 列出所有跟踪仓库
pub fn list_tracked_repos(conn: &Connection) -> Result<Vec<TrackedRepo>> {
    tracing::debug!("[list_tracked_repos] 查询所有仓库");
    let mut stmt = conn.prepare(
        "SELECT id, path, name, added_at FROM tracked_repos ORDER BY added_at DESC, id DESC"
    )?;

    let items = stmt.query_map([], |row| {
        Ok(TrackedRepo {
            id: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            added_at: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<TrackedRepo>>>()?;

    info!("[list_tracked_repos] 共 {} 条记录", items.len());
    Ok(items)
}

// Tauri 命令

#[tauri::command]
pub fn add_tracked_repo_cmd(path: String, name: String) -> Result<TrackedRepo, String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    add_tracked_repo(&conn, &path, &name).map_err(|e| format!("添加仓库失败: {}", e))
}

#[tauri::command]
pub fn remove_tracked_repo_cmd(id: i64) -> Result<(), String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    remove_tracked_repo(&conn, id).map_err(|e| format!("删除仓库失败: {}", e))
}

#[tauri::command]
pub fn list_tracked_repos_cmd() -> Result<Vec<TrackedRepo>, String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    list_tracked_repos(&conn).map_err(|e| format!("查询仓库失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE tracked_repos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                added_at INTEGER NOT NULL
            );"
        ).expect("create table");
        conn
    }

    #[test]
    fn add_and_list() {
        let conn = setup_test_db();
        add_tracked_repo(&conn, "C:\\workspace\\project-a", "project-a").expect("add");
        add_tracked_repo(&conn, "C:\\workspace\\project-b", "project-b").expect("add");

        let repos = list_tracked_repos(&conn).expect("list");
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "project-b");
        assert_eq!(repos[1].name, "project-a");
    }

    #[test]
    fn duplicate_path_fails() {
        let conn = setup_test_db();
        add_tracked_repo(&conn, "C:\\workspace\\dup", "dup").expect("first add");
        let result = add_tracked_repo(&conn, "C:\\workspace\\dup", "dup2");
        assert!(result.is_err());
    }

    #[test]
    fn remove_deletes_record() {
        let conn = setup_test_db();
        let repo = add_tracked_repo(&conn, "C:\\workspace\\to-remove", "to-remove").expect("add");
        remove_tracked_repo(&conn, repo.id).expect("remove");

        let repos = list_tracked_repos(&conn).expect("list");
        assert!(repos.is_empty());
    }

    #[test]
    fn serde_camel_case_roundtrip() {
        let repo = TrackedRepo {
            id: 1,
            path: "C:\\test".to_string(),
            name: "test".to_string(),
            added_at: 1718668800,
        };
        let json = serde_json::to_string(&repo).expect("serialize");
        assert!(json.contains("addedAt"));
        assert!(!json.contains("added_at"));

        let parsed: TrackedRepo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.name, "test");
    }
}
