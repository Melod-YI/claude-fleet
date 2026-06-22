// src-tauri/src/utils/git/worktree.rs
// worktree 业务逻辑：创建、列表、解析

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::{branch_exists, execute_git, get_repo_name, get_repo_parent, normalize_path};
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
    result = result.replace(['<', '>', ':', '"', '|', '?', '*', '/', '\\'], "-");
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
            current_path = normalize_path(path);
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

    // 3. 计算目标目录（绝对路径）
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

    // 8. 创建 worktree
    execute_git(&opts.repo_path, &["worktree", "add", &worktree_dir.to_string_lossy(), &opts.branch])
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
        .as_millis() as i64;

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
        assert_eq!(entries[0].path, normalize_path("C:/workspace/myproject"));
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
        assert_eq!(entries[0].path, normalize_path("/path/to/repo"));
    }

    #[test]
    fn parse_porcelain_empty_fails() {
        let result = parse_worktree_porcelain("");
        assert!(result.is_err());
    }
}
