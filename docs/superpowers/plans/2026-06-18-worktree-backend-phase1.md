# Git Worktree Backend (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add backend support for creating and listing git worktrees in Claude Fleet, with SQLite persistence and real-time git status fusion.

**Architecture:** Four new modules layered cleanly: `utils/git/mod.rs` (generic git commands) → `utils/git/worktree.rs` (worktree business logic) → `db/worktrees.rs` (SQLite CRUD) → `commands/worktree.rs` (Tauri invoke handlers). All functions accept `repo_path: &Path` and use `git -C` to avoid changing process directories.

**Tech Stack:** Rust, Tauri 2.0, rusqlite (bundled), serde/serde_json, tracing

---

### Task 1: Database Schema — Add worktrees Table

**Files:**
- Modify: `src-tauri/src/db/schema.rs`

- [ ] **Step 1: Add the worktrees table creation to init_tables()**

In `src-tauri/src/db/schema.rs`, add the worktrees table to the `execute_batch` call. The existing batch creates 4 tables; append the 5th:

```rust
// In init_tables(), append to the existing conn.execute_batch() string:
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
    );"
)?;
```

The `pinned`/`pinned_at` migration block below the batch stays unchanged.

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/db/schema.rs
git commit -m "feat(db): add worktrees table to schema init"
```

---

### Task 2: Database CRUD — db/worktrees.rs

**Files:**
- Create: `src-tauri/src/db/worktrees.rs`
- Modify: `src-tauri/src/db/mod.rs`

- [ ] **Step 1: Create the worktrees.rs module**

Create `src-tauri/src/db/worktrees.rs` with the full implementation:

```rust
// src-tauri/src/db/worktrees.rs
// worktrees 表 CRUD 操作

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::db::schema::get_connection;

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
```

- [ ] **Step 2: Register the module in db/mod.rs**

In `src-tauri/src/db/mod.rs`, add the new module:

```rust
pub mod worktrees;
```

Add it after the existing `pub mod migration;` line.

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: `Finished` with no errors (may show unused warnings — those are fine, they'll be resolved when commands use them).

- [ ] **Step 4: Add unit tests for the CRUD functions**

Add tests to the bottom of `src-tauri/src/db/worktrees.rs`:

```rust
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
```

- [ ] **Step 5: Run the tests**

Run: `cd src-tauri && cargo test db::worktrees -- --nocapture`
Expected: All 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db/worktrees.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): add worktrees CRUD module with tests"
```

---

### Task 3: Git Utility Layer — utils/git/mod.rs

**Files:**
- Create: `src-tauri/src/utils/git/mod.rs`
- Modify: `src-tauri/src/utils/mod.rs`

- [ ] **Step 1: Create the git module directory and file**

Create the directory `src-tauri/src/utils/git/` and create `src-tauri/src/utils/git/mod.rs`:

```rust
// src-tauri/src/utils/git/mod.rs
// 通用 git 命令封装层

pub mod worktree;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// 远程仓库信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

/// 在指定仓库目录执行 git 命令。
/// 通过 `git -C <repo_path>` 执行，无需改变进程目录。
/// 成功返回 stdout（trim），失败返回包含 stderr 的错误信息。
pub fn execute_git(repo_path: &Path, args: &[&str]) -> Result<String, String> {
    let display_args = args.join(" ");
    debug!("[execute_git] git -C {} {}", repo_path.display(), display_args);

    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("无法执行 git 命令: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("[execute_git] 成功: {} bytes", stdout.len());
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        warn!("[execute_git] 失败: git {} -> {}", display_args, stderr);
        Err(format!("git 命令失败: {}", stderr))
    }
}

/// 从远程 URL 提取仓库名称。
/// 支持:
///   https://github.com/user/repo.git
///   git@github.com:user/repo.git
///   https://gitlab.com/user/repo
pub fn extract_repo_name_from_url(url: &str) -> Option<String> {
    let url = url.trim();
    let url = url.strip_suffix(".git").unwrap_or(url);

    // SSH URLs: git@github.com:user/repo
    if url.starts_with("git@") {
        return url
            .split(':')
            .nth(1)
            .and_then(|path| path.split('/').next_back())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }

    // HTTP(S) URLs and file paths
    url.split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// 获取仓库名称。优先从 remote URL 提取，回退到目录名。
pub fn get_repo_name(repo_path: &Path) -> Result<String, String> {
    if let Ok(remote_url) = execute_git(repo_path, &["remote", "get-url", "origin"]) {
        if let Some(name) = extract_repo_name_from_url(&remote_url) {
            return Ok(name);
        }
    }

    // 回退到目录名
    repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "无法获取仓库名称".to_string())
}

/// 获取远程仓库列表
pub fn get_remotes(repo_path: &Path) -> Result<Vec<RemoteInfo>, String> {
    let output = execute_git(repo_path, &["remote", "-v"])?;
    let mut remotes = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            if seen.insert(name.clone()) {
                remotes.push(RemoteInfo {
                    name,
                    url: parts[1].to_string(),
                });
            }
        }
    }

    info!("[get_remotes] 共 {} 个远程仓库", remotes.len());
    Ok(remotes)
}

/// 获取本地分支列表
pub fn get_local_branches(repo_path: &Path) -> Result<Vec<String>, String> {
    let output = execute_git(repo_path, &["branch", "--list", "--format=%(refname:short)"])?;
    let branches: Vec<String> = output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    info!("[get_local_branches] 共 {} 个本地分支", branches.len());
    Ok(branches)
}

/// 获取远程分支列表
pub fn get_remote_branches(repo_path: &Path) -> Result<Vec<String>, String> {
    let output = execute_git(repo_path, &["branch", "-r", "--format=%(refname:short)"])?;
    let branches: Vec<String> = output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.contains("->"))
        .collect();
    info!("[get_remote_branches] 共 {} 个远程分支", branches.len());
    Ok(branches)
}

/// 检测默认分支。优先级：
/// 1. git symbolic-ref refs/remotes/origin/HEAD
/// 2. 检查常见分支名（main, master, develop）的远程引用
/// 3. 回退 "main"
pub fn get_default_branch(repo_path: &Path) -> Result<String, String> {
    // 方法 1: symbolic-ref
    if let Ok(output) = execute_git(repo_path, &["symbolic-ref", "refs/remotes/origin/HEAD"]) {
        if let Some(branch) = output.strip_prefix("refs/remotes/origin/") {
            let branch = branch.trim().to_string();
            if !branch.is_empty() {
                return Ok(branch);
            }
        }
    }

    // 方法 2: 检查常见分支的远程引用
    for candidate in ["main", "master", "develop"] {
        let ref_name = format!("refs/remotes/origin/{}", candidate);
        if execute_git(repo_path, &["rev-parse", "--verify", "--quiet", &ref_name]).is_ok() {
            return Ok(candidate.to_string());
        }
    }

    // 方法 3: 回退
    Ok("main".to_string())
}

/// 检查本地分支是否存在
pub fn branch_exists(repo_path: &Path, branch: &str) -> bool {
    let ref_name = format!("refs/heads/{}", branch);
    execute_git(repo_path, &["show-ref", "--verify", "--quiet", &ref_name]).is_ok()
}

/// 获取仓库的父目录。
/// 对于主仓库：返回 repo_path 的父目录。
/// 对于 worktree：通过 git-common-dir 定位主仓库，再取其父目录。
pub fn get_repo_parent(repo_path: &Path) -> Result<PathBuf, String> {
    let common_dir = execute_git(repo_path, &["rev-parse", "--git-common-dir"])?;
    let git_dir = execute_git(repo_path, &["rev-parse", "--git-dir"])?;

    let repo_root = if common_dir != git_dir {
        // 在 worktree 中：common_dir 指向主仓库的 .git
        let common_path = Path::new(&common_dir);
        if common_path.file_name().map_or(false, |n| n == ".git") {
            common_path
                .parent()
                .ok_or_else(|| "无法获取主仓库目录".to_string())?
        } else {
            Path::new(&common_dir)
        }
    } else {
        // 在主仓库中
        let toplevel = execute_git(repo_path, &["rev-parse", "--show-toplevel"])?;
        Path::new(&toplevel)
    };

    repo_root
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "无法获取仓库父目录".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_name_from_https_url() {
        assert_eq!(
            extract_repo_name_from_url("https://github.com/user/my-repo.git"),
            Some("my-repo".to_string())
        );
    }

    #[test]
    fn extract_name_from_https_url_without_git_suffix() {
        assert_eq!(
            extract_repo_name_from_url("https://gitlab.com/user/my-repo"),
            Some("my-repo".to_string())
        );
    }

    #[test]
    fn extract_name_from_ssh_url() {
        assert_eq!(
            extract_repo_name_from_url("git@github.com:user/my-repo.git"),
            Some("my-repo".to_string())
        );
    }

    #[test]
    fn extract_name_from_url_with_dots() {
        assert_eq!(
            extract_repo_name_from_url("https://github.com/user/repo-with-dots.v2.git"),
            Some("repo-with-dots.v2".to_string())
        );
    }

    #[test]
    fn extract_name_returns_none_for_empty() {
        assert_eq!(extract_repo_name_from_url(""), None);
        assert_eq!(extract_repo_name_from_url(".git"), None);
    }
}
```

- [ ] **Step 2: Register the git module in utils/mod.rs**

In `src-tauri/src/utils/mod.rs`, add:

```rust
pub mod git;
```

Add it after the existing `pub mod launch;` line.

- [ ] **Step 3: Run the tests**

Run: `cd src-tauri && cargo test utils::git::tests -- --nocapture`
Expected: All 5 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/utils/git/mod.rs src-tauri/src/utils/mod.rs
git commit -m "feat(git): add git utility layer with execute_git and repo info functions"
```

---

### Task 4: Worktree Business Logic — utils/git/worktree.rs

**Files:**
- Create: `src-tauri/src/utils/git/worktree.rs`

- [ ] **Step 1: Create the worktree module with types, sanitize, and porcelain parser**

Create `src-tauri/src/utils/git/worktree.rs`:

```rust
// src-tauri/src/utils/git/worktree.rs
// worktree 业务逻辑：创建、列表、解析

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use super::{execute_git, branch_exists, get_repo_name, get_repo_parent};
use crate::db::worktrees::WorktreeInfo;

/// 创建 worktree 的参数
#[derive(Debug, Clone)]
pub struct CreateWorktreeOptions {
    pub repo_path: PathBuf,
    pub name: String,
    pub branch: String,
    pub base_ref: String,
}

/// git worktree list --porcelain 的解析结果（仅包含 git 原始数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeEntry {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_main: bool,
}

/// 清理名称用于目录名：替换 Windows 非法字符为 `-`，去除首尾空格和点。
pub fn sanitize_name(name: &str) -> String {
    let mut result = name.trim().to_string();
    for ch in ['<', '>', ':', '"', '|', '?', '*', '/', '\\'] {
        result = result.replace(ch, "-");
    }
    result.trim_matches('.').to_string()
}

/// 解析 `git worktree list --porcelain` 输出。
///
/// 格式示例：
/// ```text
/// worktree /path/to/repo
/// HEAD abc123def456
/// branch refs/heads/main
///
/// worktree /path/to/worktree
/// HEAD 789ghi012jkl
/// branch refs/heads/feature
/// ```
pub fn parse_worktree_porcelain(output: &str) -> Result<Vec<GitWorktreeEntry>, String> {
    let mut entries = Vec::new();
    let mut current_path = String::new();
    let mut current_head = String::new();
    let mut current_branch: Option<String> = None;
    let mut current_bare = false;
    let mut is_first = true;

    for line in output.lines() {
        let line = line.trim();

        if line.is_empty() {
            // 空行表示一个条目结束
            if !current_path.is_empty() {
                entries.push(GitWorktreeEntry {
                    path: current_path.clone(),
                    head: current_head.clone(),
                    branch: current_branch.clone(),
                    is_bare: current_bare,
                    is_main: is_first,
                });
                is_first = false;
            }
            current_path.clear();
            current_head.clear();
            current_branch = None;
            current_bare = false;
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = path.to_string();
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current_head = head.to_string();
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // "refs/heads/feature-x" → "feature-x"
            current_branch = branch_ref
                .strip_prefix("refs/heads/")
                .map(|s| s.to_string());
        } else if line == "bare" {
            current_bare = true;
        } else if line == "detached" {
            // detached HEAD, branch stays None
        }
    }

    // 处理最后一个条目（文件末尾可能没有空行）
    if !current_path.is_empty() {
        entries.push(GitWorktreeEntry {
            path: current_path,
            head: current_head,
            branch: current_branch,
            is_bare: current_bare,
            is_main: is_first,
        });
    }

    if entries.is_empty() {
        return Err("git worktree list 输出为空".to_string());
    }

    Ok(entries)
}

/// 从 git 实时获取 worktree 列表
pub fn list_worktrees_live(repo_path: &Path) -> Result<Vec<GitWorktreeEntry>, String> {
    let output = execute_git(repo_path, &["worktree", "list", "--porcelain"])?;
    parse_worktree_porcelain(&output)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败: {}", e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("复制文件失败: {}", e))?;
        }
    }
    Ok(())
}

/// 复制主仓库的 .claude 目录到 worktree
fn copy_claude_dir(src_repo: &Path, dst_worktree: &Path) -> Result<(), String> {
    let src = src_repo.join(".claude");
    if !src.exists() {
        info!("[copy_claude_dir] 主仓库无 .claude 目录，跳过");
        return Ok(());
    }
    let dst = dst_worktree.join(".claude");
    info!("[copy_claude_dir] 复制 {} -> {}", src.display(), dst.display());
    copy_dir_recursive(&src, &dst)?;
    info!("[copy_claude_dir] 复制完成");
    Ok(())
}

/// 创建 worktree
pub fn create_worktree(opts: &CreateWorktreeOptions) -> Result<WorktreeInfo, String> {
    info!("[create_worktree] 开始: name={}, branch={}, base_ref={}",
          opts.name, opts.branch, opts.base_ref);

    // 1. 验证 repo_path 是有效的 git 仓库
    execute_git(&opts.repo_path, &["rev-parse", "--is-inside-work-tree"])
        .map_err(|e| format!("无效的 git 仓库: {}", e))?;

    // 2. 获取 repo_name
    let repo_name = get_repo_name(&opts.repo_path)?;

    // 3. 计算目标目录
    let parent = get_repo_parent(&opts.repo_path)?;
    let sanitized_name = sanitize_name(&opts.name);
    let worktree_base = parent.join(format!("{}.worktrees", repo_name));
    let worktree_dir = worktree_base.join(&sanitized_name);

    info!("[create_worktree] 目标目录: {}", worktree_dir.display());

    // 4. 检查目录冲突
    if worktree_dir.exists() {
        return Err(format!("目录已存在: {}", worktree_dir.display()));
    }

    // 5. 检查 git worktree 是否已有同名
    if let Ok(entries) = list_worktrees_live(&opts.repo_path) {
        let dir_str = worktree_dir.to_string_lossy();
        if entries.iter().any(|e| e.path == dir_str.as_ref()) {
            return Err(format!("git worktree 已存在: {}", worktree_dir.display()));
        }
    }

    // 6. 创建分支（如果不存在）
    if !branch_exists(&opts.repo_path, &opts.branch) {
        info!("[create_worktree] 创建新分支: {} from {}", opts.branch, opts.base_ref);
        execute_git(&opts.repo_path, &["branch", &opts.branch, &opts.base_ref])
            .map_err(|e| format!("创建分支失败: {}", e))?;
    } else {
        info!("[create_worktree] 分支已存在: {}", opts.branch);
    }

    // 7. 确保 worktree 根目录存在
    fs::create_dir_all(&worktree_base)
        .map_err(|e| format!("创建 worktree 根目录失败: {}", e))?;

    // 8. 创建 worktree（使用相对路径）
    let relative_path = format!("../{}.worktrees/{}", repo_name, sanitized_name);
    execute_git(&opts.repo_path, &["worktree", "add", &relative_path, &opts.branch])
        .map_err(|e| format!("创建 worktree 失败: {}", e))?;

    info!("[create_worktree] worktree 创建成功: {}", worktree_dir.display());

    // 9. 复制 .claude 目录
    if let Err(e) = copy_claude_dir(&opts.repo_path, &worktree_dir) {
        warn!("[create_worktree] 复制 .claude 目录失败（非致命）: {}", e);
    }

    // 10. 构造 WorktreeInfo（id 在数据库插入后更新）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let info = WorktreeInfo {
        id: 0, // 插入后由数据库分配
        name: opts.name.clone(),
        branch: opts.branch.clone(),
        path: worktree_dir.to_string_lossy().to_string(),
        repo_name,
        repo_path: opts.repo_path.to_string_lossy().to_string(),
        base_ref: opts.base_ref.clone(),
        created_at: now,
    };

    info!("[create_worktree] 完成: {}", info.path);
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_windows_illegal_chars() {
        assert_eq!(sanitize_name("feature:auth"), "feature-auth");
        assert_eq!(sanitize_name("foo<bar>"), "foo-bar-");
        assert_eq!(sanitize_name("a/b\\c"), "a-b-c");
    }

    #[test]
    fn sanitize_trims_whitespace_and_dots() {
        assert_eq!(sanitize_name("  hello  "), "hello");
        assert_eq!(sanitize_name(".hidden."), "hidden");
    }

    #[test]
    fn sanitize_handles_clean_name() {
        assert_eq!(sanitize_name("feature-auth"), "feature-auth");
    }

    #[test]
    fn parse_porcelain_single_entry() {
        let output = "\
worktree C:/workspace/myproject
HEAD abc123def456789
branch refs/heads/main
";
        let entries = parse_worktree_porcelain(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "C:/workspace/myproject");
        assert_eq!(entries[0].head, "abc123def456789");
        assert_eq!(entries[0].branch, Some("main".to_string()));
        assert!(entries[0].is_main);
        assert!(!entries[0].is_bare);
    }

    #[test]
    fn parse_porcelain_multiple_entries() {
        let output = "\
worktree C:/workspace/myproject
HEAD aaa111
branch refs/heads/main

worktree C:/workspace/myproject.worktrees/feature-x
HEAD bbb222
branch refs/heads/feature-x

worktree C:/workspace/myproject.worktrees/detached-wt
HEAD ccc333
detached
";
        let entries = parse_worktree_porcelain(output).unwrap();
        assert_eq!(entries.len(), 3);

        assert!(entries[0].is_main);
        assert_eq!(entries[0].branch, Some("main".to_string()));

        assert!(!entries[1].is_main);
        assert_eq!(entries[1].branch, Some("feature-x".to_string()));

        assert!(!entries[2].is_main);
        assert_eq!(entries[2].branch, None);
    }

    #[test]
    fn parse_porcelain_no_trailing_newline() {
        let output = "\
worktree /path/to/repo
HEAD abc123
branch refs/heads/main";
        let entries = parse_worktree_porcelain(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/path/to/repo");
    }

    #[test]
    fn parse_porcelain_empty_fails() {
        let result = parse_worktree_porcelain("");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cd src-tauri && cargo test utils::git::worktree -- --nocapture`
Expected: All 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/utils/git/worktree.rs
git commit -m "feat(git): add worktree business logic with create, list, and porcelain parser"
```

---

### Task 5: Tauri Commands — commands/worktree.rs

**Files:**
- Create: `src-tauri/src/commands/worktree.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create the worktree commands module**

Create `src-tauri/src/commands/worktree.rs`:

```rust
// src-tauri/src/commands/worktree.rs
// Worktree 相关 Tauri 命令

use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

use crate::db::schema::get_connection;
use crate::db::worktrees::{
    WorktreeInfo, insert_worktree, list_worktrees_by_repo,
};
use crate::utils::git::{
    RemoteInfo, get_repo_name, get_remotes, get_local_branches,
    get_remote_branches, get_default_branch,
};
use crate::utils::git::worktree::{
    CreateWorktreeOptions, GitWorktreeEntry, create_worktree, list_worktrees_live,
};

/// Worktree 状态标记
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeStatus {
    Active,
    Missing,
    Unmanaged,
}

/// 列表返回的 worktree 条目（融合数据库 + git 实时数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeListItem {
    pub id: Option<i64>,
    pub name: String,
    pub repo_name: String,
    pub base_ref: Option<String>,
    pub created_at: Option<i64>,
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub is_main: bool,
    pub status: WorktreeStatus,
}

/// 仓库信息（供前端构建分支选择器）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,
    pub remotes: Vec<RemoteInfo>,
    pub local_branches: Vec<String>,
    pub remote_branches: Vec<String>,
    pub default_branch: String,
}

/// 创建 worktree
#[tauri::command]
pub fn create_worktree_cmd(
    repo_path: String,
    name: String,
    branch: String,
    base_ref: String,
) -> Result<WorktreeInfo, String> {
    info!("[create_worktree_cmd] 开始: repo={}, name={}, branch={}, base_ref={}",
          repo_path, name, branch, base_ref);

    let opts = CreateWorktreeOptions {
        repo_path: Path::new(&repo_path).to_path_buf(),
        name: name.clone(),
        branch: branch.clone(),
        base_ref: base_ref.clone(),
    };

    // 1. 执行 git 操作创建 worktree
    let mut info = create_worktree(&opts)?;

    // 2. 持久化到数据库
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    insert_worktree(&conn, &info).map_err(|e| format!("数据库插入失败: {}", e))?;

    // 3. 获取数据库分配的 id
    if let Ok(Some(db_info)) = crate::db::worktrees::get_worktree_by_path(&conn, &info.path) {
        info.id = db_info.id;
    }

    info!("[create_worktree_cmd] 完成: id={}, path={}", info.id, info.path);
    Ok(info)
}

/// 列表 worktree（融合数据库 + git 实时状态）
#[tauri::command]
pub fn list_worktrees_cmd(
    repo_path: String,
) -> Result<Vec<WorktreeListItem>, String> {
    info!("[list_worktrees_cmd] 开始: repo={}", repo_path);

    let path = Path::new(&repo_path);
    let repo_name = get_repo_name(path).unwrap_or_else(|_| "unknown".to_string());

    // 1. 获取 git 实时数据
    let git_items = list_worktrees_live(path)?;

    // 2. 获取数据库记录
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    let db_items = list_worktrees_by_repo(&conn, &repo_path)
        .map_err(|e| format!("数据库查询失败: {}", e))?;

    // 3. 融合
    let mut db_map: HashMap<String, &WorktreeInfo> = HashMap::new();
    for item in &db_items {
        db_map.insert(item.path.clone(), item);
    }

    let git_paths: HashSet<String> = git_items.iter().map(|e| e.path.clone()).collect();

    let mut results: Vec<WorktreeListItem> = Vec::new();

    // 遍历 git 实时数据（跳过主仓库）
    for git_entry in &git_items {
        if git_entry.is_main {
            continue;
        }

        if let Some(db_info) = db_map.get(&git_entry.path) {
            // Active: 数据库有 + git 有
            results.push(WorktreeListItem {
                id: Some(db_info.id),
                name: db_info.name.clone(),
                repo_name: db_info.repo_name.clone(),
                base_ref: Some(db_info.base_ref.clone()),
                created_at: Some(db_info.created_at),
                path: git_entry.path.clone(),
                head: git_entry.head.clone(),
                branch: git_entry.branch.clone(),
                is_main: false,
                status: WorktreeStatus::Active,
            });
        } else {
            // Unmanaged: git 有但数据库没有
            let name = extract_name_from_path(&git_entry.path);
            results.push(WorktreeListItem {
                id: None,
                name,
                repo_name: repo_name.clone(),
                base_ref: None,
                created_at: None,
                path: git_entry.path.clone(),
                head: git_entry.head.clone(),
                branch: git_entry.branch.clone(),
                is_main: false,
                status: WorktreeStatus::Unmanaged,
            });
        }
    }

    // 遍历数据库记录，找出 Missing 项
    for db_info in &db_items {
        if !git_paths.contains(&db_info.path) {
            results.push(WorktreeListItem {
                id: Some(db_info.id),
                name: db_info.name.clone(),
                repo_name: db_info.repo_name.clone(),
                base_ref: Some(db_info.base_ref.clone()),
                created_at: Some(db_info.created_at),
                path: db_info.path.clone(),
                head: String::new(),
                branch: Some(db_info.branch.clone()),
                is_main: false,
                status: WorktreeStatus::Missing,
            });
        }
    }

    // 排序：Active 优先，再按 created_at 降序
    results.sort_by(|a, b| {
        let status_order = |s: &WorktreeStatus| match s {
            WorktreeStatus::Active => 0,
            WorktreeStatus::Unmanaged => 1,
            WorktreeStatus::Missing => 2,
        };
        let sa = status_order(&a.status);
        let sb = status_order(&b.status);
        sa.cmp(&sb).then_with(|| {
            b.created_at.unwrap_or(0).cmp(&a.created_at.unwrap_or(0))
        })
    });

    info!("[list_worktrees_cmd] 完成: {} 个 worktree (active={}, unmanaged={}, missing={})",
          results.len(),
          results.iter().filter(|r| matches!(r.status, WorktreeStatus::Active)).count(),
          results.iter().filter(|r| matches!(r.status, WorktreeStatus::Unmanaged)).count(),
          results.iter().filter(|r| matches!(r.status, WorktreeStatus::Missing)).count(),
    );

    Ok(results)
}

/// 获取仓库信息
#[tauri::command]
pub fn get_repo_info_cmd(
    repo_path: String,
) -> Result<RepoInfo, String> {
    info!("[get_repo_info_cmd] 开始: repo={}", repo_path);

    let path = Path::new(&repo_path);

    let name = get_repo_name(path)?;
    let remotes = get_remotes(path)?;
    let local_branches = get_local_branches(path)?;
    let remote_branches = get_remote_branches(path)?;
    let default_branch = get_default_branch(path)?;

    let info = RepoInfo {
        name,
        remotes,
        local_branches,
        remote_branches,
        default_branch,
    };

    info!("[get_repo_info_cmd] 完成: name={}, remotes={}, local={}, remote={}",
          info.name, info.remotes.len(), info.local_branches.len(), info.remote_branches.len());

    Ok(info)
}

/// 从路径中提取目录名作为 worktree 名称
fn extract_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
```

- [ ] **Step 2: Register the module in commands/mod.rs**

In `src-tauri/src/commands/mod.rs`, add:

```rust
pub mod worktree;
```

The file currently has 4 modules. It should now have 5:

```rust
pub mod session;
pub mod session_commands;
pub mod terminal;
pub mod sound;
pub mod worktree;
```

- [ ] **Step 3: Register the Tauri commands in lib.rs**

In `src-tauri/src/lib.rs`, add the import and register the commands:

Add the import (near the existing command imports):

```rust
use commands::worktree::{create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd};
```

Add to the `invoke_handler` `generate_handler!` macro (append before the closing `]`):

```rust
            // Worktree commands
            create_worktree_cmd,
            list_worktrees_cmd,
            get_repo_info_cmd,
```

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: `Finished` with no errors. Warnings about unused imports in worktree.rs are OK at this stage.

- [ ] **Step 5: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass (existing 11 + new 11 = 22 tests total).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/worktree.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add worktree Tauri commands (create, list, get_repo_info)"
```

---

### Task 6: Frontend TypeScript Types

**Files:**
- Create: `src/types/worktree.ts`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Create the worktree type definitions**

Create `src/types/worktree.ts`:

```typescript
// src/types/worktree.ts
// Worktree 相关类型定义

export interface RemoteInfo {
  name: string;
  url: string;
}

export interface RepoInfo {
  name: string;
  remotes: RemoteInfo[];
  localBranches: string[];
  remoteBranches: string[];
  defaultBranch: string;
}

export interface WorktreeInfo {
  id: number;
  name: string;
  branch: string;
  path: string;
  repoName: string;
  repoPath: string;
  baseRef: string;
  createdAt: number;
}

export type WorktreeStatus = 'active' | 'missing' | 'unmanaged';

export interface WorktreeListItem {
  id: number | null;
  name: string;
  repoName: string;
  baseRef: string | null;
  createdAt: number | null;
  path: string;
  head: string;
  branch: string | null;
  isMain: boolean;
  status: WorktreeStatus;
}
```

- [ ] **Step 2: Export from types/index.ts**

In `src/types/index.ts`, add the re-export:

```typescript
export * from './worktree';
```

Add it after the existing exports.

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src/types/worktree.ts src/types/index.ts
git commit -m "feat(types): add worktree TypeScript type definitions"
```

---

### Task 7: Final Verification & Cleanup

**Files:**
- No new files

- [ ] **Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: All 22 tests pass.

- [ ] **Step 2: Run TypeScript type check**

Run: `npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 3: Full cargo check (release mode)**

Run: `cd src-tauri && cargo check --release`
Expected: `Finished` with no errors.

- [ ] **Step 4: Verify new file structure**

Confirm these files exist:

```
src-tauri/src/utils/git/mod.rs
src-tauri/src/utils/git/worktree.rs
src-tauri/src/db/worktrees.rs
src-tauri/src/commands/worktree.rs
src/types/worktree.ts
```

- [ ] **Step 5: Final commit (if any cleanup needed)**

```bash
git add -A
git commit -m "chore: final cleanup for worktree phase 1 backend"
```
