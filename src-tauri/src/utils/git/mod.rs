// src-tauri/src/utils/git/mod.rs
// 通用 git 命令封装层

pub mod worktree;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// 归一化路径分隔符。
/// Git 在 Windows 上输出正斜杠（C:/path），而 Rust PathBuf 使用反斜杠（C:\path）。
/// 在路径进入系统时调用此函数，统一为平台原生格式。
pub fn normalize_path(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        path.replace('/', "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string()
    }
}

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

    let output = crate::utils::process::command("git")
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
    let common_dir = normalize_path(&execute_git(repo_path, &["rev-parse", "--git-common-dir"])?);
    let git_dir = normalize_path(&execute_git(repo_path, &["rev-parse", "--git-dir"])?);

    let repo_root: PathBuf = if common_dir != git_dir {
        // 在 worktree 中：common_dir 指向主仓库的 .git
        let common_path = PathBuf::from(&common_dir);
        if common_path.file_name().is_some_and(|n| n == ".git") {
            common_path
                .parent()
                .ok_or_else(|| "无法获取主仓库目录".to_string())?
                .to_path_buf()
        } else {
            common_path
        }
    } else {
        // 在主仓库中
        let toplevel = normalize_path(&execute_git(repo_path, &["rev-parse", "--show-toplevel"])?);
        PathBuf::from(&toplevel)
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
