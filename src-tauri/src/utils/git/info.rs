// src-tauri/src/utils/git/info.rs
// 采集工作目录的 git 概要信息，供"运行中"卡片展示。

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

use super::{
    execute_git, get_current_branch, get_dirty_file_count, get_last_commit,
    get_upstream_ahead_behind, is_worktree,
};

/// 工作目录的 git 概要信息。
/// snake_case 序列化，与 RunningSession 保持一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,                // 分支名；detached 时为短 sha
    pub is_detached: bool,
    pub is_worktree: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_name: Option<String>,  // worktree 目录名（cwd 末段）；非 worktree 时为 None
    pub ahead: u32,                    // 领先上游提交数
    pub behind: u32,                   // 落后上游提交数
    pub dirty: bool,                   // 是否有未提交更改
    pub last_commit_sha: String,       // 短 hash
    pub last_commit_message: String,   // 最近提交信息（截断至 60 字符）
}

/// 采集 cwd 的 git 信息。非 git 仓库返回 `None`。
pub fn gather_git_info(cwd: &Path) -> Option<GitInfo> {
    info!("[gather_git_info] 开始: cwd={}", cwd.display());

    // 1. 判断是否 git 仓库
    let inside = execute_git(cwd, &["rev-parse", "--is-inside-work-tree"]).unwrap_or_default();
    if inside.trim() != "true" {
        info!("[gather_git_info] 非 git 仓库: {}", cwd.display());
        return None;
    }

    let is_wt = is_worktree(cwd);
    let worktree_name = if is_wt {
        cwd.file_name().map(|s| s.to_string_lossy().to_string())
    } else {
        None
    };

    // 2. 分支（detached 时回退短 sha）
    let (branch_opt, is_detached) = match get_current_branch(cwd) {
        Ok(v) => v,
        Err(e) => {
            warn!("[gather_git_info] 获取分支失败: {}", e);
            (None, false)
        }
    };
    let branch = branch_opt.unwrap_or_else(|| {
        execute_git(cwd, &["rev-parse", "--short", "HEAD"])
            .unwrap_or_else(|_| "unknown".to_string())
    });

    // 3. dirty
    let dirty = match get_dirty_file_count(cwd) {
        Ok(n) => n > 0,
        Err(e) => {
            warn!("[gather_git_info] 获取 dirty 状态失败: {}", e);
            false
        }
    };

    // 4. ahead/behind（无上游则 0/0）
    let (ahead, behind) = get_upstream_ahead_behind(cwd);

    // 5. 最近提交
    let (sha, message) = match get_last_commit(cwd) {
        Ok(v) => v,
        Err(e) => {
            warn!("[gather_git_info] 获取最近提交失败: {}", e);
            (String::new(), String::new())
        }
    };
    let message = truncate_chars(&message, 60);

    let result = GitInfo {
        branch,
        is_detached,
        is_worktree: is_wt,
        worktree_name,
        ahead,
        behind,
        dirty,
        last_commit_sha: sha,
        last_commit_message: message,
    };
    info!("[gather_git_info] 完成: branch={}, detached={}, worktree={}, dirty={}, ahead={}, behind={}",
          result.branch, result.is_detached, result.is_worktree,
          result.dirty, result.ahead, result.behind);
    Some(result)
}

/// 按字符数截断（避免中文字符被切半）。
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::git::test_helpers;

    #[test]
    fn gather_returns_none_for_non_git_dir() {
        let dir = test_helpers::unique_temp_dir("gi-nongit");
        assert!(gather_git_info(&dir).is_none());
    }

    #[test]
    fn gather_on_clean_main_repo() {
        let repo = test_helpers::init_repo("gi-clean");
        let info = gather_git_info(&repo).expect("应返回 Some");
        assert!(!info.is_detached);
        assert!(!info.is_worktree);
        assert!(!info.dirty, "干净仓库 dirty 应为 false");
        assert_eq!((info.ahead, info.behind), (0, 0));
        assert!(!info.branch.is_empty());
        assert_eq!(info.last_commit_message, "initial commit");
        assert!(!info.last_commit_sha.is_empty());
    }

    #[test]
    fn gather_dirty_when_uncommitted() {
        let repo = test_helpers::init_repo("gi-dirty");
        std::fs::write(repo.join("uncommitted.txt"), "x\n").unwrap();
        let info = gather_git_info(&repo).expect("应返回 Some");
        assert!(info.dirty, "有未提交文件 dirty 应为 true");
    }

    #[test]
    fn gather_detached_branch_is_short_sha() {
        let repo = test_helpers::init_repo("gi-det");
        let status = crate::utils::process::command("git")
            .arg("-C").arg(&repo)
            .args(["checkout", "--detach", "HEAD"])
            .status().unwrap();
        assert!(status.success(), "git checkout --detach 失败");
        let info = gather_git_info(&repo).expect("应返回 Some");
        assert!(info.is_detached);
        assert_eq!(info.branch, info.last_commit_sha, "detached 时 branch 应为短 sha");
    }

    #[test]
    fn gather_worktree_name_is_last_segment() {
        let main = test_helpers::init_repo("gi-wt-name");
        let wt_path = test_helpers::unique_temp_path("gi-wt-name-linked");
        let status = crate::utils::process::command("git")
            .arg("-C").arg(&main)
            .args(["worktree", "add", &wt_path.to_string_lossy(), "-b", "feature-y"])
            .status().unwrap();
        assert!(status.success(), "git worktree add 失败");
        let info = gather_git_info(&wt_path).expect("应返回 Some");
        assert!(info.is_worktree);
        let expected = wt_path.file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(info.worktree_name.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn gather_worktree_name_none_for_main_repo() {
        let repo = test_helpers::init_repo("gi-wt-none");
        let info = gather_git_info(&repo).expect("应返回 Some");
        assert!(!info.is_worktree);
        assert!(info.worktree_name.is_none(), "非 worktree 时 worktree_name 应为 None");
    }

    #[test]
    fn truncate_handles_multibyte() {
        assert_eq!(truncate_chars("abcdef", 3), "abc");
        assert_eq!(truncate_chars("你好世界", 2), "你好");
        assert_eq!(truncate_chars("ab", 5), "ab");
    }
}
