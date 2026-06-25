// src-tauri/src/commands/worktree.rs
// Worktree 相关 Tauri 命令

use std::collections::{HashMap, HashSet};
use std::path::Path;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::db::schema::get_connection;
use crate::db::worktrees::{
    WorktreeInfo, insert_worktree, list_worktrees_by_repo, get_worktree_by_path,
    delete_worktree_by_path,
};
use crate::utils::git::{
    RemoteInfo, get_repo_name, get_remotes, get_local_branches,
    get_remote_branches, get_default_branch, get_ahead_behind, get_dirty_file_count,
    branch_exists, is_branch_merged,
};
use crate::utils::git::worktree::{
    CreateWorktreeOptions, create_worktree, delete_worktree, list_worktrees_live,
};

/// Worktree 状态标记
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub uncommitted_changes: Option<u32>,
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

/// 删除 worktree 前的安全预检结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionSafety {
    pub is_managed: bool,
    pub will_delete_branch: bool,
    pub uncommitted_changes: u32,
    pub unmerged_commits: u32,
    pub blocked: bool,
    pub reasons: Vec<String>,
}

/// 纯逻辑：根据各项检查值计算 blocked 与 reasons。
/// - 未提交变更 > 0 → 阻断（无论是否托管）
/// - will_delete_branch 且 unmerged > 0 → 阻断
pub fn compute_deletion_safety_fields(
    uncommitted: u32,
    unmerged: u32,
    will_delete_branch: bool,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if uncommitted > 0 {
        reasons.push(format!("{} 个未提交变更", uncommitted));
    }
    if will_delete_branch && unmerged > 0 {
        reasons.push(format!("{} 个未合并到主干的提交", unmerged));
    }
    let blocked = !reasons.is_empty();
    (blocked, reasons)
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

    if name.trim().is_empty() {
        return Err("worktree 名称不能为空".to_string());
    }
    if branch.trim().is_empty() {
        return Err("分支名不能为空".to_string());
    }
    if base_ref.trim().is_empty() {
        return Err("基点引用不能为空".to_string());
    }

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
    if let Ok(Some(db_info)) = get_worktree_by_path(&conn, &info.path) {
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

        // 获取 git 状态（仅 Active worktree，best-effort）
        let mut ahead: Option<u32> = None;
        let mut behind: Option<u32> = None;
        let mut uncommitted_changes: Option<u32> = None;

        let wt_path = Path::new(&git_entry.path);
        if let Some(branch_name) = &git_entry.branch {
            // ahead/behind：需要 branch + base_ref
            if let Some(db_info) = db_map.get(&git_entry.path) {
                if let Ok((a, b)) = get_ahead_behind(wt_path, branch_name, &db_info.base_ref) {
                    ahead = Some(a);
                    behind = Some(b);
                }
            }
            // dirty files
            if let Ok(count) = get_dirty_file_count(wt_path) {
                uncommitted_changes = Some(count);
            }
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
                ahead,
                behind,
                uncommitted_changes,
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
                ahead,
                behind,
                uncommitted_changes,
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
                ahead: None,
                behind: None,
                uncommitted_changes: None,
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

/// 删除 worktree（git 清理 + 数据库删除）
#[tauri::command]
pub fn delete_worktree_cmd(
    path: String,
    repo_path: String,
    branch: Option<String>,
    delete_branch: bool,
) -> Result<(), String> {
    info!("[delete_worktree_cmd] 开始: path={}, repo={}, branch={:?}, delete_branch={}",
          path, repo_path, branch, delete_branch);

    // 1. Git 清理（worktree remove + 可选 branch delete）
    delete_worktree(
        Path::new(&repo_path),
        &path,
        branch.as_deref(),
        delete_branch,
    )?;

    // 2. 删除数据库记录
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    delete_worktree_by_path(&conn, &path)
        .map_err(|e| format!("数据库删除失败: {}", e))?;

    info!("[delete_worktree_cmd] 完成: {}", path);
    Ok(())
}

/// 删除 worktree 前的安全预检
#[tauri::command]
pub fn preflight_delete_worktree_cmd(
    path: String,
    repo_path: String,
    branch: Option<String>,
) -> Result<DeletionSafety, String> {
    info!("[preflight_delete_worktree_cmd] 开始: path={}, repo={}, branch={:?}",
          path, repo_path, branch);

    let repo = Path::new(&repo_path);
    let wt_path = Path::new(&path);

    // 1. 是否托管（DB 有记录）
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    let is_managed = get_worktree_by_path(&conn, &path)
        .map_err(|e| format!("数据库查询失败: {}", e))?
        .is_some();
    info!("[preflight_delete_worktree_cmd] is_managed={}", is_managed);

    // 2. 是否会删分支
    let will_delete_branch = is_managed
        && branch.as_ref().is_some_and(|b| branch_exists(repo, b));
    info!("[preflight_delete_worktree_cmd] will_delete_branch={}", will_delete_branch);

    // 3. 未提交变更（托管/未托管都查）
    let uncommitted_changes = match get_dirty_file_count(wt_path) {
        Ok(n) => n,
        Err(e) => {
            warn!("[preflight_delete_worktree_cmd] 获取未提交变更失败，按 0 处理: {}", e);
            0
        }
    };

    // 4. 未合并提交（仅 will_delete_branch 时查）
    let unmerged_commits = if will_delete_branch {
        if let Some(b) = &branch {
            match get_default_branch(repo) {
                Ok(main_branch) => match is_branch_merged(repo, b, &main_branch) {
                    Ok((_, n)) => n,
                    Err(e) => {
                        warn!("[preflight_delete_worktree_cmd] 合并检查失败，按 0 处理: {}", e);
                        0
                    }
                },
                Err(e) => {
                    warn!("[preflight_delete_worktree_cmd] 获取默认分支失败，按 0 处理: {}", e);
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    };

    let (blocked, reasons) =
        compute_deletion_safety_fields(uncommitted_changes, unmerged_commits, will_delete_branch);

    info!("[preflight_delete_worktree_cmd] 完成: uncommitted={}, unmerged={}, blocked={}, reasons={:?}",
          uncommitted_changes, unmerged_commits, blocked, reasons);

    Ok(DeletionSafety {
        is_managed,
        will_delete_branch,
        uncommitted_changes,
        unmerged_commits,
        blocked,
        reasons,
    })
}

/// 纯逻辑：统计 live worktree 数（排除主仓库）
pub fn count_live_worktrees(entries: &[crate::utils::git::worktree::GitWorktreeEntry]) -> u32 {
    entries.iter().filter(|e| !e.is_main).count() as u32
}

/// 轻量计数：1 次 git porcelain + 1 次 DB 查询，供仓库折叠徽标使用
#[tauri::command]
pub fn count_worktrees_cmd(repo_path: String) -> Result<u32, String> {
    info!("[count_worktrees_cmd] 开始: repo={}", repo_path);

    let path = Path::new(&repo_path);

    // 1. live 列表（只取一次，同时用于计数与 missing 比对）
    let live_entries = match list_worktrees_live(path) {
        Ok(entries) => entries,
        Err(e) => {
            warn!("[count_worktrees_cmd] 获取 live worktree 失败，按 0 处理: {}", e);
            Vec::new()
        }
    };
    let live = count_live_worktrees(&live_entries);
    let live_paths: std::collections::HashSet<String> =
        live_entries.iter().map(|e| e.path.clone()).collect();

    // 2. missing 计数（DB 有但 live 没有）
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    let db_items = list_worktrees_by_repo(&conn, &repo_path)
        .map_err(|e| format!("数据库查询失败: {}", e))?;
    let missing = db_items.iter().filter(|d| !live_paths.contains(&d.path)).count() as u32;

    let total = live + missing;
    info!("[count_worktrees_cmd] 完成: live={}, missing={}, total={}", live, missing, total);
    Ok(total)
}

/// 从路径中提取目录名作为 worktree 名称
fn extract_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_status_serializes_to_lowercase() {
        let active = serde_json::to_string(&WorktreeStatus::Active).unwrap();
        assert_eq!(active, "\"active\"");

        let missing = serde_json::to_string(&WorktreeStatus::Missing).unwrap();
        assert_eq!(missing, "\"missing\"");

        let unmanaged = serde_json::to_string(&WorktreeStatus::Unmanaged).unwrap();
        assert_eq!(unmanaged, "\"unmanaged\"");
    }

    #[test]
    fn worktree_status_deserializes_from_lowercase() {
        let active: WorktreeStatus = serde_json::from_str("\"active\"").unwrap();
        assert!(matches!(active, WorktreeStatus::Active));

        let missing: WorktreeStatus = serde_json::from_str("\"missing\"").unwrap();
        assert!(matches!(missing, WorktreeStatus::Missing));

        let unmanaged: WorktreeStatus = serde_json::from_str("\"unmanaged\"").unwrap();
        assert!(matches!(unmanaged, WorktreeStatus::Unmanaged));
    }

    #[test]
    fn worktree_list_item_camel_case_roundtrip() {
        let item = WorktreeListItem {
            id: Some(1),
            name: "test".to_string(),
            repo_name: "myrepo".to_string(),
            base_ref: Some("origin/main".to_string()),
            created_at: Some(1718668800),
            path: "/path/to/wt".to_string(),
            head: "abc123".to_string(),
            branch: Some("feature".to_string()),
            is_main: false,
            status: WorktreeStatus::Active,
            ahead: Some(3),
            behind: Some(1),
            uncommitted_changes: Some(2),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("repoName"));
        assert!(json.contains("baseRef"));
        assert!(json.contains("createdAt"));
        assert!(json.contains("isMain"));
        assert!(json.contains("uncommittedChanges"));
        assert!(!json.contains("repo_name"));

        let parsed: WorktreeListItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.repo_name, "myrepo");
        assert!(matches!(parsed.status, WorktreeStatus::Active));
    }

    #[test]
    fn repo_info_camel_case_roundtrip() {
        let info = RepoInfo {
            name: "myrepo".to_string(),
            remotes: vec![RemoteInfo { name: "origin".to_string(), url: "https://github.com/user/repo.git".to_string() }],
            local_branches: vec!["main".to_string()],
            remote_branches: vec!["origin/main".to_string()],
            default_branch: "main".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("localBranches"));
        assert!(json.contains("remoteBranches"));
        assert!(json.contains("defaultBranch"));
        assert!(!json.contains("local_branches"));

        let parsed: RepoInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "myrepo");
        assert_eq!(parsed.remotes.len(), 1);
    }

    #[test]
    fn deletion_safety_camel_case_roundtrip() {
        let s = DeletionSafety {
            is_managed: true,
            will_delete_branch: true,
            uncommitted_changes: 2,
            unmerged_commits: 3,
            blocked: true,
            reasons: vec!["未提交".to_string()],
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("isManaged"));
        assert!(json.contains("willDeleteBranch"));
        assert!(json.contains("uncommittedChanges"));
        assert!(json.contains("unmergedCommits"));
        assert!(!json.contains("uncommitted_changes"));
    }

    #[test]
    fn compute_safety_not_blocked_when_clean() {
        let (blocked, reasons) = compute_deletion_safety_fields(0, 0, true);
        assert!(!blocked);
        assert!(reasons.is_empty());
    }

    #[test]
    fn compute_safety_blocked_by_uncommitted() {
        let (blocked, reasons) = compute_deletion_safety_fields(5, 0, true);
        assert!(blocked);
        assert!(reasons.iter().any(|r| r.contains("未提交") && r.contains("5")));
    }

    #[test]
    fn compute_safety_blocked_by_unmerged_when_will_delete_branch() {
        let (blocked, reasons) = compute_deletion_safety_fields(0, 2, true);
        assert!(blocked);
        assert!(reasons.iter().any(|r| r.contains("未合并") && r.contains("2")));
    }

    #[test]
    fn compute_safety_ignores_unmerged_when_not_deleting_branch() {
        // 未托管：will_delete_branch=false，unmerged 不应阻断
        let (blocked, _reasons) = compute_deletion_safety_fields(0, 2, false);
        assert!(!blocked);
    }

    #[test]
    fn compute_safety_blocked_by_both_lists_both_reasons() {
        let (blocked, reasons) = compute_deletion_safety_fields(1, 1, true);
        assert!(blocked);
        assert_eq!(reasons.len(), 2);
    }

    use crate::utils::git::worktree::GitWorktreeEntry;

    #[test]
    fn count_live_excludes_main() {
        let entries = vec![
            GitWorktreeEntry { path: "/r".into(), head: "a".into(), branch: Some("main".into()), is_bare: false, is_main: true },
            GitWorktreeEntry { path: "/r.w/feat".into(), head: "b".into(), branch: Some("feat".into()), is_bare: false, is_main: false },
            GitWorktreeEntry { path: "/r.w/det".into(), head: "c".into(), branch: None, is_bare: false, is_main: false },
        ];
        assert_eq!(count_live_worktrees(&entries), 2);
    }

    #[test]
    fn count_live_zero_when_only_main() {
        let entries = vec![
            GitWorktreeEntry { path: "/r".into(), head: "a".into(), branch: Some("main".into()), is_bare: false, is_main: true },
        ];
        assert_eq!(count_live_worktrees(&entries), 0);
    }
}
