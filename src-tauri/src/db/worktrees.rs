// src-tauri/src/db/worktrees.rs
// worktrees 表 CRUD 操作

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Worktree 数据库记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
    pub id: i64,
    pub name: String,
    pub branch: String,
    pub path: String,
    pub repo_name: String,
    pub repo_path: String,
    pub base_ref: String,
    pub created_at: i64,
}

/// 插入 worktree 记录。path 有 UNIQUE 约束，重复插入会报错。
pub fn insert_worktree(conn: &Connection, info: &WorktreeInfo) -> Result<()> {
    info!("[insert_worktree] 插入 worktree: name={}, path={}", info.name, info.path);
    conn.execute(
        "INSERT INTO worktrees (name, branch, path, repo_name, repo_path, base_ref, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            info.name,
            info.branch,
            info.path,
            info.repo_name,
            info.repo_path,
            info.base_ref,
            info.created_at,
        ],
    )?;
    info!("[insert_worktree] 成功插入 worktree");
    Ok(())
}

/// 按主仓库路径查询所有 worktree
pub fn list_worktrees_by_repo(conn: &Connection, repo_path: &str) -> Result<Vec<WorktreeInfo>> {
    info!("[list_worktrees_by_repo] 查询 repo_path={}", repo_path);
    let mut stmt = conn.prepare(
        "SELECT id, name, branch, path, repo_name, repo_path, base_ref, created_at
         FROM worktrees WHERE repo_path = ?1 ORDER BY created_at DESC"
    )?;

    let items = stmt.query_map(params![repo_path], |row| {
        Ok(WorktreeInfo {
            id: row.get(0)?,
            name: row.get(1)?,
            branch: row.get(2)?,
            path: row.get(3)?,
            repo_name: row.get(4)?,
            repo_path: row.get(5)?,
            base_ref: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?
    .collect::<Result<Vec<WorktreeInfo>>>()?;

    info!("[list_worktrees_by_repo] 共 {} 条记录", items.len());
    Ok(items)
}

/// 按 worktree 路径查询单条记录
pub fn get_worktree_by_path(conn: &Connection, path: &str) -> Result<Option<WorktreeInfo>> {
    info!("[get_worktree_by_path] 查询 path={}", path);
    let mut stmt = conn.prepare(
        "SELECT id, name, branch, path, repo_name, repo_path, base_ref, created_at
         FROM worktrees WHERE path = ?1"
    )?;

    let mut rows = stmt.query_map(params![path], |row| {
        Ok(WorktreeInfo {
            id: row.get(0)?,
            name: row.get(1)?,
            branch: row.get(2)?,
            path: row.get(3)?,
            repo_name: row.get(4)?,
            repo_path: row.get(5)?,
            base_ref: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    match rows.next() {
        Some(Ok(info)) => {
            info!("[get_worktree_by_path] 找到记录: id={}", info.id);
            Ok(Some(info))
        }
        Some(Err(e)) => Err(e),
        None => {
            info!("[get_worktree_by_path] 未找到记录");
            Ok(None)
        }
    }
}

/// 按路径删除 worktree 记录（第二期删除功能使用）
pub fn delete_worktree_by_path(conn: &Connection, path: &str) -> Result<()> {
    info!("[delete_worktree_by_path] 删除 path={}", path);
    conn.execute("DELETE FROM worktrees WHERE path = ?1", params![path])?;
    info!("[delete_worktree_by_path] 成功删除");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE worktrees (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                branch TEXT NOT NULL,
                path TEXT NOT NULL UNIQUE,
                repo_name TEXT NOT NULL,
                repo_path TEXT NOT NULL,
                base_ref TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );"
        ).expect("create table");
        conn
    }

    fn sample_worktree(path: &str) -> WorktreeInfo {
        WorktreeInfo {
            id: 0,
            name: "feature-x".to_string(),
            branch: "feature-x".to_string(),
            path: path.to_string(),
            repo_name: "myproject".to_string(),
            repo_path: "C:\\workspace\\myproject".to_string(),
            base_ref: "origin/main".to_string(),
            created_at: 1718668800,
        }
    }

    #[test]
    fn insert_and_query_by_path() {
        let conn = setup_test_db();
        let wt = sample_worktree("C:\\workspace\\myproject.worktrees\\feature-x");
        insert_worktree(&conn, &wt).expect("insert");

        let result = get_worktree_by_path(&conn, &wt.path).expect("query");
        assert!(result.is_some());
        let found = result.unwrap();
        assert_eq!(found.name, "feature-x");
        assert_eq!(found.branch, "feature-x");
        assert_eq!(found.base_ref, "origin/main");
        assert!(found.id > 0);
    }

    #[test]
    fn list_by_repo_returns_matching_records() {
        let conn = setup_test_db();
        let wt1 = sample_worktree("C:\\workspace\\myproject.worktrees\\wt1");
        let mut wt2 = sample_worktree("C:\\workspace\\myproject.worktrees\\wt2");
        wt2.name = "feature-y".to_string();
        wt2.branch = "feature-y".to_string();

        insert_worktree(&conn, &wt1).expect("insert wt1");
        insert_worktree(&conn, &wt2).expect("insert wt2");

        let results = list_worktrees_by_repo(&conn, "C:\\workspace\\myproject").expect("list");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn list_by_repo_returns_empty_for_unknown_repo() {
        let conn = setup_test_db();
        let results = list_worktrees_by_repo(&conn, "C:\\nonexistent").expect("list");
        assert!(results.is_empty());
    }

    #[test]
    fn duplicate_path_fails() {
        let conn = setup_test_db();
        let wt = sample_worktree("C:\\workspace\\myproject.worktrees\\dup");
        insert_worktree(&conn, &wt).expect("first insert");
        let result = insert_worktree(&conn, &wt);
        assert!(result.is_err());
    }

    #[test]
    fn delete_by_path_removes_record() {
        let conn = setup_test_db();
        let wt = sample_worktree("C:\\workspace\\myproject.worktrees\\to-delete");
        insert_worktree(&conn, &wt).expect("insert");

        delete_worktree_by_path(&conn, &wt.path).expect("delete");

        let result = get_worktree_by_path(&conn, &wt.path).expect("query");
        assert!(result.is_none());
    }

    #[test]
    fn serde_camel_case_roundtrip() {
        let wt = sample_worktree("C:\\test");
        let json = serde_json::to_string(&wt).expect("serialize");
        assert!(json.contains("repoName"));
        assert!(json.contains("repoPath"));
        assert!(json.contains("baseRef"));
        assert!(json.contains("createdAt"));

        let parsed: WorktreeInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.name, wt.name);
        assert_eq!(parsed.base_ref, wt.base_ref);
    }
}
