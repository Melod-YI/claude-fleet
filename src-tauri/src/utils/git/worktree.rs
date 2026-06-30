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

    // 6. 创建分支（如果已存在则拒绝）
    if branch_exists(&opts.repo_path, &opts.branch) {
        return Err(format!("分支 \"{}\" 已存在，请更换分支名", opts.branch));
    }
    info!("[create_worktree] 创建新分支: {} from {}", opts.branch, opts.base_ref);
    // 使用 --no-track：当 base_ref 是远程跟踪引用（如 upstream/master）时，git 默认会
    // 自动把新分支配置为跟踪该远端（branch.<name>.remote=upstream），导致后续 `git push`
    // 直接推到受保护的 upstream。worktree 分支应为独立本地分支，内容取自 base_ref 但不跟踪它，
    // 后续由用户 `git push -u origin <branch>` 推送到 origin 再发起 MR。
    execute_git(&opts.repo_path, &["branch", "--no-track", &opts.branch, &opts.base_ref])
        .map_err(|e| format!("创建分支失败: {}", e))?;

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

/// 删除 worktree：移除 git worktree、可选删除分支
///
/// 步骤：
/// 1. 如果目录存在 → `git worktree remove`（带 --force 处理未提交变更）
/// 2. 如果目录不存在 → `git worktree prune`（清理残留元数据）
/// 3. 如果 `delete_branch && branch.is_some()` → `git branch -D`
///
/// 所有 git 操作 best-effort：失败仅记录警告，不中断流程。
pub fn delete_worktree(
    repo_path: &Path,
    worktree_path: &str,
    branch: Option<&str>,
    delete_branch: bool,
) -> Result<(), String> {
    info!("[delete_worktree] 开始: worktree={}, branch={:?}, delete_branch={}",
          worktree_path, branch, delete_branch);

    // 1. 移除 git worktree
    let wt_path = Path::new(worktree_path);
    if wt_path.exists() {
        match execute_git(repo_path, &["worktree", "remove", worktree_path, "--force"]) {
            Ok(_) => info!("[delete_worktree] git worktree remove 成功"),
            Err(e) => warn!("[delete_worktree] git worktree remove 失败（继续）: {}", e),
        }

        // git worktree remove 会清空内容但可能留下空目录，手动清理
        if wt_path.exists() {
            match fs::read_dir(wt_path) {
                Ok(entries) => {
                    if entries.count() == 0 {
                        if let Err(e) = fs::remove_dir(wt_path) {
                            warn!("[delete_worktree] 清理空目录失败（继续）: {}", e);
                        } else {
                            info!("[delete_worktree] 已清理空目录: {}", worktree_path);
                        }
                    } else {
                        info!("[delete_worktree] 目录非空，保留: {}", worktree_path);
                    }
                }
                Err(e) => warn!("[delete_worktree] 读取目录失败: {}", e),
            }
        }
    } else {
        info!("[delete_worktree] 目录不存在，执行 worktree prune");
        match execute_git(repo_path, &["worktree", "prune"]) {
            Ok(_) => info!("[delete_worktree] worktree prune 成功"),
            Err(e) => warn!("[delete_worktree] worktree prune 失败（继续）: {}", e),
        }
    }

    // 2. 可选删除分支
    if delete_branch {
        if let Some(branch_name) = branch {
            if branch_exists(repo_path, branch_name) {
                match execute_git(repo_path, &["branch", "-D", branch_name]) {
                    Ok(_) => info!("[delete_worktree] 分支 {} 已删除", branch_name),
                    Err(e) => warn!("[delete_worktree] 删除分支失败（继续）: {}", e),
                }
            } else {
                info!("[delete_worktree] 分支 {} 不存在，跳过", branch_name);
            }
        }
    }

    info!("[delete_worktree] 完成");
    Ok(())
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

    /// 复现"从 upstream/master 创建 worktree 时 push 误推到 upstream"的根因：
    /// 从远程跟踪引用创建分支时，git 默认自动设置上游跟踪，导致 `git push` 推到 upstream。
    /// 修复后（--no-track）新分支不应跟踪任何远端，内容仍来自 base_ref。
    #[test]
    fn create_worktree_from_remote_ref_does_not_track_upstream() {
        use crate::utils::git::get_current_branch;
        use crate::utils::git::test_helpers::{init_repo, unique_temp_path};
        use crate::utils::process::command;

        // 1. 主仓库（含初始提交），获取其默认分支名（main 或 master，因 git 版本而异）
        let main = init_repo("cwt-upstream");
        let default = get_current_branch(&main).unwrap().0.unwrap();

        // 2. 构造 bare 远端作为 upstream，推送默认分支并 fetch，建立 refs/remotes/upstream/<default>
        let bare = unique_temp_path("cwt-upstream-bare");
        let s = command("git")
            .arg("init").arg("--bare").arg(&bare)
            .status().unwrap();
        assert!(s.success(), "git init --bare 失败");
        let bare_str = bare.to_string_lossy().to_string();
        command("git").arg("-C").arg(&main)
            .args(["remote", "add", "upstream", bare_str.as_str()])
            .status().unwrap();
        command("git").arg("-C").arg(&main)
            .args(["push", "upstream", default.as_str()])
            .status().unwrap();
        command("git").arg("-C").arg(&main)
            .args(["fetch", "upstream"])
            .status().unwrap();

        // 3. 从 upstream/<default> 创建 worktree
        let base_ref = format!("upstream/{}", default);
        let opts = CreateWorktreeOptions {
            repo_path: main.clone(),
            name: "wt1".to_string(),
            branch: "feat1".to_string(),
            base_ref: base_ref.clone(),
        };
        let info = create_worktree(&opts).expect("创建 worktree 应成功");

        // 4. 新分支不应设置上游跟踪（--no-track 生效）：upstream 字段应为空
        let upstream_ref = execute_git(
            &main,
            &["for-each-ref", "--format=%(upstream)", "refs/heads/feat1"],
        )
        .unwrap();
        assert!(
            upstream_ref.is_empty(),
            "新分支不应跟踪 upstream，实际 upstream={}", upstream_ref
        );

        // 5. 内容应来自 base_ref（worktree HEAD 等于 upstream/<default>）
        let wt_head = execute_git(Path::new(&info.path), &["rev-parse", "HEAD"]).unwrap();
        let base_head = execute_git(&main, &["rev-parse", &base_ref]).unwrap();
        assert_eq!(wt_head, base_head, "worktree HEAD 应等于 base_ref 的 HEAD");

        // 清理
        let _ = std::fs::remove_dir_all(&info.path);
        let _ = std::fs::remove_dir_all(&main);
        let _ = std::fs::remove_dir_all(&bare);
    }
}
