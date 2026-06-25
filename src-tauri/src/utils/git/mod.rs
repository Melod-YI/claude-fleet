// src-tauri/src/utils/git/mod.rs
// 通用 git 命令封装层

pub mod worktree;
pub mod info;

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

/// 是否处于 git worktree 中（而非主仓库）。
/// 通过比较 `--git-dir`（当前工作区 git 目录）与 `--git-common-dir`（主仓库 .git）判断。
/// 二者不同说明当前是 worktree。
///
/// 注意：git 对子目录可能返回**相对路径**（相对 repo_path），而对仓库根返回绝对路径，
/// 直接比较字符串会因相对/绝对不一致而误判。故先将两者都解析为绝对规范路径再比较。
pub fn is_worktree(repo_path: &Path) -> bool {
    let git_dir = match execute_git(repo_path, &["rev-parse", "--git-dir"]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let common_dir = match execute_git(repo_path, &["rev-parse", "--git-common-dir"]) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let git_dir_abs = resolve_absolute(repo_path, &git_dir);
    let common_dir_abs = resolve_absolute(repo_path, &common_dir);
    git_dir_abs != common_dir_abs
}

/// 将 git 返回的路径（可能是相对 repo_path 的相对路径）解析为绝对规范路径。
/// 相对路径基于 `base` 拼接；`canonicalize` 消解 `..` 与符号链接。
/// `canonicalize` 失败时回退到拼接 + 分隔符归一化的结果。
fn resolve_absolute(base: &Path, p: &str) -> PathBuf {
    let p = p.trim();
    let pb = Path::new(p);
    let abs = if pb.is_absolute() {
        PathBuf::from(p)
    } else {
        base.join(pb)
    };
    abs.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(normalize_path(&abs.to_string_lossy())))
}

/// 获取当前分支名与 detached 状态。
/// 返回 `(分支名, is_detached)`。detached HEAD 时分支名为 `None`。
pub fn get_current_branch(repo_path: &Path) -> Result<(Option<String>, bool), String> {
    let output = execute_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if output.trim() == "HEAD" {
        Ok((None, true))
    } else {
        Ok((Some(output), false))
    }
}

/// 获取最近一次提交的短 sha 与提交信息。
/// 返回 `(短 sha, message)`。使用 `\0` 分隔以避免信息中含换行造成拆分歧义。
pub fn get_last_commit(repo_path: &Path) -> Result<(String, String), String> {
    let output = execute_git(repo_path, &["log", "-1", "--format=%h%x00%s"])?;
    let mut parts = output.splitn(2, '\u{0}');
    let sha = parts.next().unwrap_or("").trim().to_string();
    let message = parts.next().unwrap_or("").trim().to_string();
    Ok((sha, message))
}

/// 获取相对上游跟踪分支（`@{u}`）的 ahead/behind 提交数。
/// `rev-list --left-right --count @{u}...HEAD`：左侧=上游独有(behind)，右侧=本地独有(ahead)。
/// 无上游或命令失败返回 `(0, 0)`。
pub fn get_upstream_ahead_behind(repo_path: &Path) -> (u32, u32) {
    match execute_git(repo_path, &["rev-list", "--left-right", "--count", "@{u}...HEAD"]) {
        Ok(output) => {
            let parts: Vec<&str> = output.split_whitespace().collect();
            if parts.len() >= 2 {
                let behind = parts[0].parse::<u32>().unwrap_or(0);
                let ahead = parts[1].parse::<u32>().unwrap_or(0);
                (ahead, behind)
            } else {
                (0, 0)
            }
        }
        Err(e) => {
            debug!("[get_upstream_ahead_behind] 无上游或失败: {}", e);
            (0, 0)
        }
    }
}

/// 获取仓库的父目录。
/// 对于主仓库：返回 repo_path 的父目录（基于 --show-toplevel）。
/// 对于 worktree：通过 git-common-dir 定位主仓库，再取其父目录。
pub fn get_repo_parent(repo_path: &Path) -> Result<PathBuf, String> {
    let repo_root: PathBuf = if is_worktree(repo_path) {
        // 在 worktree 中：common_dir 指向主仓库的 .git
        let common_dir = normalize_path(&execute_git(repo_path, &["rev-parse", "--git-common-dir"])?);
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

/// 从 worktree 路径解析其主仓库根目录。
///
/// 用于删除 worktree 时从主仓库（而非 worktree 自身路径）执行 git 命令，
/// 避免 cwd 落在被删目录内导致 `worktree remove` Permission denied、
/// 以及目录删除后 `branch -D` / `branch_exists` 因 cwd 失效而失败。
///
/// 实现：`git -C <worktree> rev-parse --git-common-dir` 返回主仓库的 `.git`
/// 目录（worktree 场景为绝对路径），取其父目录即主仓库根。
pub fn get_main_repo_root(worktree_path: &Path) -> Result<PathBuf, String> {
    let common_dir = normalize_path(&execute_git(worktree_path, &["rev-parse", "--git-common-dir"])?);
    let common_path = PathBuf::from(&common_dir);
    let root = common_path
        .parent()
        .ok_or_else(|| "无法从 git-common-dir 解析主仓库根目录".to_string())?;
    if root.as_os_str().is_empty() {
        return Err("git-common-dir 返回相对路径，无法解析主仓库根目录".to_string());
    }
    info!("[get_main_repo_root] worktree={} -> main_repo={}",
          worktree_path.display(), root.display());
    Ok(root.to_path_buf())
}

/// 获取 worktree 相对 base_ref 的 ahead/behind 提交数。
/// `repo_path` 指向 worktree 目录，`base_ref` 可以是 `origin/main` 或 `main`。
pub fn get_ahead_behind(repo_path: &Path, branch: &str, base_ref: &str) -> Result<(u32, u32), String> {
    let remote_ref = if base_ref.starts_with("origin/") {
        base_ref.to_string()
    } else {
        format!("origin/{}", base_ref)
    };

    let range = format!("{}...{}", branch, remote_ref);
    let output = execute_git(repo_path, &["rev-list", "--left-right", "--count", &range])?;

    let parts: Vec<&str> = output.split_whitespace().collect();
    if parts.len() >= 2 {
        let ahead = parts[0].parse::<u32>().unwrap_or(0);
        let behind = parts[1].parse::<u32>().unwrap_or(0);
        Ok((ahead, behind))
    } else {
        Err(format!("无法解析 rev-list 输出: {}", output))
    }
}

/// 获取未提交变更文件数（staged + unstaged + untracked）。
/// `repo_path` 指向 worktree 目录。
pub fn get_dirty_file_count(repo_path: &Path) -> Result<u32, String> {
    let output = execute_git(repo_path, &["status", "--porcelain"])?;
    let count = output.lines().filter(|l| !l.is_empty()).count() as u32;
    Ok(count)
}

/// 解析 `git rev-list --count <range>` 输出为 u32。
/// 空或非数字返回 0。
pub fn parse_rev_list_count(output: &str) -> u32 {
    output.trim().parse::<u32>().unwrap_or(0)
}

/// 判定 branch 相对 main_branch 的合并状态。
/// 返回 (is_merged, unmerged_commits)。
/// unmerged_commits = `git rev-list --count main..branch`（branch 有而 main 没有的提交数）。
/// best-effort：git 失败时返回 (false, 0)，不阻断删除流程。
pub fn is_branch_merged(
    repo_path: &Path,
    branch: &str,
    main_branch: &str,
) -> Result<(bool, u32), String> {
    let range = format!("{}..{}", main_branch, branch);
    match execute_git(repo_path, &["rev-list", "--count", &range]) {
        Ok(output) => {
            let n = parse_rev_list_count(&output);
            Ok((n == 0, n))
        }
        Err(e) => {
            warn!("[is_branch_merged] rev-list 失败，按不阻断处理: {}", e);
            Ok((false, 0))
        }
    }
}

/// 测试辅助：创建临时 git 仓库。仅供 `#[cfg(test)]` 使用。
#[cfg(test)]
pub(crate) mod test_helpers {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// 创建唯一临时目录（不初始化 git）
    pub fn unique_temp_dir(prefix: &str) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir()
            .join(format!("claude-fleet-test-{}-{}-{}", prefix, pid, id));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 返回唯一临时路径（不创建目录）。用于需由 git 自行创建的路径（如 `git worktree add`）。
    pub fn unique_temp_path(prefix: &str) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir()
            .join(format!("claude-fleet-test-{}-{}-{}", prefix, pid, id))
    }

    /// 在 path 执行 git 命令（用 process::command 避免 Windows 弹窗）
    fn git(path: &Path, args: &[&str]) {
        let status = crate::utils::process::command("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .status()
            .expect("git 命令执行失败");
        assert!(status.success(), "git {:?} 在 {} 失败", args, path.display());
    }

    /// 初始化一个 git 仓库（含 config 与初始提交），返回仓库路径
    pub fn init_repo(prefix: &str) -> PathBuf {
        let path = unique_temp_dir(prefix);
        git(&path, &["init"]);
        git(&path, &["config", "user.email", "test@example.com"]);
        git(&path, &["config", "user.name", "Test"]);
        std::fs::write(path.join("README.md"), "init\n").unwrap();
        git(&path, &["add", "."]);
        git(&path, &["commit", "-m", "initial commit"]);
        path
    }

    /// 在仓库内做一次提交（写文件 + add + commit）
    pub fn commit(path: &Path, name: &str, msg: &str) {
        std::fs::write(path.join(name), format!("{}\n", msg)).unwrap();
        git(path, &["add", "."]);
        git(path, &["commit", "-m", msg]);
    }
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

    #[test]
    fn is_worktree_false_for_main_repo() {
        let repo = test_helpers::init_repo("iwt-main");
        assert!(!is_worktree(&repo));
    }

    #[test]
    fn is_worktree_false_for_subdir_of_main_repo() {
        // 主干仓库的子目录：git-dir 返回绝对路径、common-dir 返回相对路径，
        // 必须消解相对性后再比较，否则会误判为 worktree。
        let repo = test_helpers::init_repo("iwt-subdir");
        let subdir = repo.join("nested").join("deep");
        std::fs::create_dir_all(&subdir).unwrap();
        assert!(!is_worktree(&subdir), "主干仓库的子目录不应被识别为 worktree");
    }

    #[test]
    fn is_worktree_true_for_worktree() {
        let main = test_helpers::init_repo("iwt-wt");
        let wt_path = test_helpers::unique_temp_path("iwt-wt-linked");
        // 在主仓库中创建一个 worktree
        let status = crate::utils::process::command("git")
            .arg("-C")
            .arg(&main)
            .args(["worktree", "add", &wt_path.to_string_lossy(), "-b", "feature-x"])
            .status()
            .expect("git worktree add 失败");
        assert!(status.success(), "git worktree add 失败");
        assert!(is_worktree(&wt_path));
    }

    #[test]
    fn get_current_branch_on_main_repo() {
        let repo = test_helpers::init_repo("gcb-main");
        let (branch, is_detached) = get_current_branch(&repo).unwrap();
        assert!(!is_detached);
        assert!(branch.is_some(), "应返回分支名");
        let name = branch.unwrap();
        assert!(!name.is_empty() && name != "HEAD");
    }

    #[test]
    fn get_current_branch_detached() {
        let repo = test_helpers::init_repo("gcb-det");
        let status = crate::utils::process::command("git")
            .arg("-C").arg(&repo)
            .args(["checkout", "--detach", "HEAD"])
            .status().unwrap();
        assert!(status.success(), "git checkout --detach 失败");
        let (branch, is_detached) = get_current_branch(&repo).unwrap();
        assert!(is_detached);
        assert!(branch.is_none(), "detached 时分支名应为 None");
    }

    #[test]
    fn get_last_commit_returns_initial() {
        let repo = test_helpers::init_repo("glc-init");
        let (sha, msg) = get_last_commit(&repo).unwrap();
        assert!(!sha.is_empty());
        assert_eq!(msg, "initial commit");
    }

    #[test]
    fn get_upstream_ahead_behind_no_upstream_returns_zero() {
        let repo = test_helpers::init_repo("uab-noup");
        // 全新本地仓库无上游，应返回 (0,0) 而非报错
        let (ahead, behind) = get_upstream_ahead_behind(&repo);
        assert_eq!((ahead, behind), (0, 0));
    }

    #[test]
    fn get_upstream_ahead_behind_divergent() {
        let repo = test_helpers::init_repo("uab-div");
        // 创建 upstream 分支（与当前分支同处于初始提交）
        let s = crate::utils::process::command("git")
            .arg("-C").arg(&repo)
            .args(["branch", "upstream"])
            .status().unwrap();
        assert!(s.success(), "git branch upstream 失败");
        // 切到 upstream，加 3 个提交（behind 来源）
        let s = crate::utils::process::command("git")
            .arg("-C").arg(&repo)
            .args(["checkout", "upstream"])
            .status().unwrap();
        assert!(s.success(), "git checkout upstream 失败");
        test_helpers::commit(&repo, "u1.txt", "u1");
        test_helpers::commit(&repo, "u2.txt", "u2");
        test_helpers::commit(&repo, "u3.txt", "u3");
        // 切回原默认分支，设置其上游为本地 upstream 分支
        let s = crate::utils::process::command("git")
            .arg("-C").arg(&repo)
            .args(["checkout", "-"])
            .status().unwrap();
        assert!(s.success(), "git checkout - 失败");
        let s = crate::utils::process::command("git")
            .arg("-C").arg(&repo)
            .args(["branch", "--set-upstream-to=upstream"])
            .status().unwrap();
        assert!(s.success(), "git branch --set-upstream-to 失败");
        // 在当前分支加 2 个提交（ahead 来源）
        test_helpers::commit(&repo, "d1.txt", "d1");
        test_helpers::commit(&repo, "d2.txt", "d2");
        // 当前分支领先 2、落后 3
        let (ahead, behind) = get_upstream_ahead_behind(&repo);
        assert_eq!((ahead, behind), (2, 3), "应 ahead=2(本地领先), behind=3(上游领先)");
    }

    #[test]
    fn parse_rev_list_count_parses_number() {
        assert_eq!(parse_rev_list_count("3"), 3);
    }

    #[test]
    fn parse_rev_list_count_trims_whitespace() {
        assert_eq!(parse_rev_list_count("  12 \n"), 12);
    }

    #[test]
    fn parse_rev_list_count_zero_when_empty() {
        assert_eq!(parse_rev_list_count(""), 0);
    }

    #[test]
    fn parse_rev_list_count_zero_when_non_numeric() {
        assert_eq!(parse_rev_list_count("abc"), 0);
    }
}
