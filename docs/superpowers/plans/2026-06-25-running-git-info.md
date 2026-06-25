# 运行中页面显示 Git 信息 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在"运行中"页面每个 session 卡片上显示其工作目录的 git 信息（分支、是否 worktree、ahead/behind、dirty、最近提交），非 git 仓库不显示。

**Architecture:** 后端在 `utils/git/` 模块新增 `info.rs` 采集 git 信息（复用现有 `execute_git`/`get_dirty_file_count`/`is_worktree`），`RunningSession` 增加 `git_info` 字段；转入等待输入/首次加入/手动刷新时后台线程非阻塞采集，经现有 `running_sessions_changed` 事件下发；前端 `SessionCardNew` 新增独立 git 行。

**Tech Stack:** Rust + Tauri 2 + tracing；React + TypeScript + Tailwind。测试用 `cargo test`（需本机 git 可执行）。

**参考 spec：** `docs/superpowers/specs/2026-06-25-running-git-info-design.md`

---

## 文件结构

| 文件 | 责任 | 动作 |
|---|---|---|
| `src-tauri/src/utils/git/mod.rs` | 通用 git 命令封装 | 修改：抽 `is_worktree`、重构 `get_repo_parent`、新增 3 个小工具、新增 `pub mod info;`、新增测试 helper |
| `src-tauri/src/utils/git/info.rs` | `GitInfo` 结构 + `gather_git_info` 编排 | 新建 |
| `src-tauri/src/utils/running_sessions.rs` | 运行中 session 状态管理 | 修改：`RunningSession` 加 `git_info` 字段、新增 `refresh_git_info_background` + 去重缓存 |
| `src-tauri/src/utils/sessions_watcher.rs` | 文件监听 | 修改：在 create/转入等待态处触发后台采集 |
| `src-tauri/src/commands/session.rs` | Tauri 命令 | 修改：新增 `refresh_git_info_all` 命令 |
| `src-tauri/src/lib.rs` | 命令注册 | 修改：注册新命令 |
| `src/types/session.ts` | 前端类型 | 修改：新增 `GitInfo`、`RunningSession.git_info` |
| `src/components/running/SessionCardNew.tsx` | 卡片组件 | 修改：新增 git 行 |
| `src/components/running/RunningTab.tsx` | 运行中页 | 修改：刷新按钮触发 `refresh_git_info_all` |

---

### Task 1: 抽取 `is_worktree` 并重构 `get_repo_parent`

**Files:**
- Modify: `src-tauri/src/utils/git/mod.rs:179-207`
- Modify: `src-tauri/src/utils/git/mod.rs`（新增 `is_worktree` 函数 + 测试 helper 模块）

- [ ] **Step 1: 在 `git/mod.rs` 测试模块上方新增 `is_worktree` 函数**

在 `get_repo_parent` 函数之前插入：

```rust
/// 是否处于 git worktree 中（而非主仓库）。
/// 通过比较 `--git-common-dir`（主仓库 .git）与 `--git-dir`（当前工作区 .git）判断。
/// 二者不同说明当前是 worktree。
pub fn is_worktree(repo_path: &Path) -> bool {
    let common = execute_git(repo_path, &["rev-parse", "--git-common-dir"]);
    let git_dir = execute_git(repo_path, &["rev-parse", "--git-dir"]);
    match (common, git_dir) {
        (Ok(c), Ok(g)) => normalize_path(&c) != normalize_path(&g),
        _ => false,
    }
}
```

- [ ] **Step 2: 重构 `get_repo_parent` 复用 `is_worktree`**

将 `get_repo_parent`（179-207 行）整体替换为：

```rust
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
```

- [ ] **Step 3: 新增共享测试 helper 模块（供本任务及后续任务使用）**

在 `git/mod.rs` 末尾的 `#[cfg(test)] mod tests { ... }` 之前插入新的测试 helper 模块：

```rust
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
```

- [ ] **Step 4: 在 `git/mod.rs` 的 `mod tests` 内新增 `is_worktree` 测试**

在 `extract_name_returns_none_for_empty` 测试之后追加：

```rust
    #[test]
    fn is_worktree_false_for_main_repo() {
        let repo = test_helpers::init_repo("iwt-main");
        assert!(!is_worktree(&repo));
    }

    #[test]
    fn is_worktree_true_for_worktree() {
        let main = test_helpers::init_repo("iwt-wt");
        let wt_path = main.parent().unwrap().join("iwt-wt-linked");
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
```

- [ ] **Step 5: 运行测试验证通过**

Run: `cd src-tauri && cargo test --lib git::tests::is_worktree -- --nocapture`
Expected: 2 个测试 PASS。

- [ ] **Step 6: 编译并跑全量 git 模块测试，确认未破坏现有**

Run: `cd src-tauri && cargo test --lib git::`
Expected: 全部 PASS（含原有 `extract_name_*`、`parse_porcelain_*` 等）。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/utils/git/mod.rs
git commit -m "refactor: 抽取 is_worktree 并复用于 get_repo_parent"
```

---

### Task 2: 新增 git 小工具函数

**Files:**
- Modify: `src-tauri/src/utils/git/mod.rs`（新增 `get_current_branch`/`get_last_commit`/`get_upstream_ahead_behind` + 测试）

- [ ] **Step 1: 在 `is_worktree` 函数之后新增三个工具函数**

```rust
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
```

> `debug!` 已在 `git/mod.rs` 顶部导入（`use tracing::{debug, info, warn};`）。

- [ ] **Step 2: 在 `mod tests` 内新增测试**

```rust
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
```

- [ ] **Step 3: 运行测试验证通过**

Run: `cd src-tauri && cargo test --lib git::tests::get_ -- --nocapture`
Expected: 4 个测试 PASS。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/utils/git/mod.rs
git commit -m "feat: 新增 get_current_branch/get_last_commit/get_upstream_ahead_behind"
```

---

### Task 3: 新建 `git/info.rs` — `GitInfo` 与 `gather_git_info`

**Files:**
- Create: `src-tauri/src/utils/git/info.rs`
- Modify: `src-tauri/src/utils/git/mod.rs`（注册子模块）

- [ ] **Step 1: 在 `git/mod.rs` 顶部 `pub mod worktree;` 旁注册子模块**

将 `pub mod worktree;` 改为：

```rust
pub mod worktree;
pub mod info;
```

- [ ] **Step 2: 新建 `src-tauri/src/utils/git/info.rs`，写入结构体与编排函数**

```rust
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

    // 2. 分支（detached 时回退短 sha）
    let (branch_opt, is_detached) = get_current_branch(cwd).unwrap_or((None, true));
    let branch = branch_opt.unwrap_or_else(|| {
        execute_git(cwd, &["rev-parse", "--short", "HEAD"])
            .unwrap_or_else(|_| "unknown".to_string())
    });

    // 3. dirty
    let dirty = get_dirty_file_count(cwd).map(|n| n > 0).unwrap_or(false);

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
    fn truncate_handles_multibyte() {
        assert_eq!(truncate_chars("abcdef", 3), "abc");
        assert_eq!(truncate_chars("你好世界", 2), "你好");
        assert_eq!(truncate_chars("ab", 5), "ab");
    }
}
```

- [ ] **Step 3: 运行测试验证通过**

Run: `cd src-tauri && cargo test --lib git::info -- --nocapture`
Expected: 5 个测试 PASS。

- [ ] **Step 4: 跑全量测试确认未破坏**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/utils/git/info.rs src-tauri/src/utils/git/mod.rs
git commit -m "feat: 新增 GitInfo 与 gather_git_info 采集逻辑"
```

---

### Task 4: `RunningSession` 增加 `git_info` 字段

**Files:**
- Modify: `src-tauri/src/utils/running_sessions.rs:34-50`（结构体）
- Modify: `src-tauri/src/utils/running_sessions.rs:184-195`（构造）

- [ ] **Step 1: 在 `running_sessions.rs` 顶部导入 `GitInfo`**

在现有 `use crate::utils::claude_session::{...};` 之后追加：

```rust
use crate::utils::git::info::GitInfo;
```

- [ ] **Step 2: 给 `RunningSession` 结构体加字段**

在 `custom_name` 字段之后（`pub custom_name: Option<String>,` 之后、结构体闭合 `}` 之前）追加：

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_info: Option<GitInfo>,  // 工作目录 git 概要信息
```

- [ ] **Step 3: 在 `add_running_session_from_file` 的构造体中初始化 `git_info`**

在 `let session = RunningSession { ... }`（约 184 行）中，`custom_name: None,` 之后追加：

```rust
        git_info: None,
```

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo build`
Expected: 编译成功（可能伴随现有 `dead_code` warning 无关项）。

- [ ] **Step 5: 跑测试确认未破坏**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/utils/running_sessions.rs
git commit -m "feat: RunningSession 增加 git_info 字段"
```

---

### Task 5: `refresh_git_info_background` 后台采集编排与去重缓存

**Files:**
- Modify: `src-tauri/src/utils/running_sessions.rs`（新增去重缓存 + 函数）

- [ ] **Step 1: 在 `running_sessions.rs` 顶部补充导入 `Path` 与 `info::gather_git_info`**

将 Task 4 加入的 `use crate::utils::git::info::GitInfo;` 改为：

```rust
use crate::utils::git::info::{gather_git_info, GitInfo};
use std::path::Path;
```

> `thread`、`Instant`、`Lazy`、`Mutex`、`HashMap`、`tauri::Emitter` 已在文件顶部导入。

- [ ] **Step 2: 新增去重缓存常量与静态**

在 `AWAY_SUMMARY_CACHE` 静态定义之后追加：

```rust
/// git 信息后台采集的去重缓存：cwd -> 上次触发 Instant。
/// 同一 cwd 在 GIT_REFRESH_DEDUPE_SECS 内仅触发一次（自动触发场景）。
static GIT_REFRESH_CACHE: Lazy<Mutex<HashMap<String, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// 自动触发去重窗口（秒）。
const GIT_REFRESH_DEDUPE_SECS: u64 = 5;
```

- [ ] **Step 3: 新增 `refresh_git_info_background` 函数**

在 `remove_running_session_by_pid` 函数之后追加：

```rust
/// 后台采集并更新指定 session 的 git 信息，非阻塞。
/// - `force = true`：绕过去重（手动刷新）。
/// - `force = false`：受 `GIT_REFRESH_DEDUPE_SECS` 去重约束（自动触发）。
/// 采集完成后写回 `RUNNING_SESSIONS` 并 emit `running_sessions_changed`。
pub fn refresh_git_info_background(pid: u32, app_handle: tauri::AppHandle, force: bool) {
    info!("[refresh_git_info_background] 触发: pid={}, force={}", pid, force);

    thread::spawn(move || {
        // 1. 读取 cwd（退出锁，避免长持有）
        let cwd = {
            let sessions = RUNNING_SESSIONS.lock().unwrap();
            match sessions.get(&pid) {
                Some(s) => s.cwd.clone(),
                None => {
                    info!("[refresh_git_info_background] pid={} 不存在，跳过", pid);
                    return;
                }
            }
        };

        // 2. 去重（仅自动触发）
        if !force {
            let now = Instant::now();
            let should_skip = {
                let mut cache = GIT_REFRESH_CACHE.lock().unwrap();
                if let Some(last) = cache.get(&cwd) {
                    if now.duration_since(*last).as_secs() < GIT_REFRESH_DEDUPE_SECS {
                        true
                    } else {
                        cache.insert(cwd.clone(), now);
                        false
                    }
                } else {
                    cache.insert(cwd.clone(), now);
                    false
                }
            };
            if should_skip {
                debug!("[refresh_git_info_background] cwd={} 在 {}s 内已触发，跳过",
                       cwd, GIT_REFRESH_DEDUPE_SECS);
                return;
            }
        }

        // 3. 采集
        let git_info = gather_git_info(Path::new(&cwd));

        // 4. 写回
        {
            let mut sessions = RUNNING_SESSIONS.lock().unwrap();
            if let Some(s) = sessions.get_mut(&pid) {
                s.git_info = git_info;
                info!("[refresh_git_info_background] 已更新 pid={} 的 git_info", pid);
            } else {
                info!("[refresh_git_info_background] pid={} 已移除，丢弃采集结果", pid);
                return;
            }
        }

        // 5. 通知前端
        let sessions = get_running_sessions();
        if let Err(e) = app_handle.emit("running_sessions_changed", sessions) {
            error!("[refresh_git_info_background] 发送事件失败: {}", e);
        } else {
            debug!("[refresh_git_info_background] 事件发送成功: pid={}", pid);
        }
    });
}
```

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo build`
Expected: 编译成功。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/utils/running_sessions.rs
git commit -m "feat: 新增 refresh_git_info_background 后台采集与去重"
```

> 说明：本函数的端到端行为依赖 `RUNNING_SESSIONS` 全局态与 `AppHandle`，难以纯单元测试；其核心依赖 `gather_git_info`（已在 Task 3 覆盖）与去重逻辑。去重为简单时间比较，不单独设测试。

---

### Task 6: watcher 触发接线（首次加入 + 转入等待态）

**Files:**
- Modify: `src-tauri/src/utils/sessions_watcher.rs:226-267`（create）
- Modify: `src-tauri/src/utils/sessions_watcher.rs:286-314`（modify）

- [ ] **Step 1: 在 `sessions_watcher.rs` 导入 `refresh_git_info_background`**

在 `use crate::utils::running_sessions::{...};`（13-15 行附近）的导入列表中加入 `refresh_git_info_background`：

```rust
    add_running_session_from_file,
    update_session_status_from_file,
    refresh_git_info_background,
```

- [ ] **Step 2: 在 `handle_session_create` 首次加入后触发采集**

在 `handle_session_create` 中，`emit_sessions_changed(app_handle);`（257 行）之后、`if session.status == "idle" ...` 块之前插入：

```rust
    // 后台采集 git 信息（首次加入）
    refresh_git_info_background(session.pid, app_handle.clone(), false);
```

- [ ] **Step 3: 在 `handle_session_modify` 转入等待态处触发采集**

在 `handle_session_modify` 的 `if is_waiting_now && !was_waiting_before { ... }` 块内，`emit_waiting_input_notification(&session, app_handle);` 之后追加：

```rust
        // 转入等待输入：agent 刚完成一轮工作，后台刷新 git 信息
        refresh_git_info_background(session.pid, app_handle.clone(), false);
```

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo build`
Expected: 编译成功。

- [ ] **Step 5: 跑全量测试确认未破坏**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/utils/sessions_watcher.rs
git commit -m "feat: session 首次加入/转入等待态时触发 git 信息采集"
```

---

### Task 7: `refresh_git_info_all` Tauri 命令

**Files:**
- Modify: `src-tauri/src/commands/session.rs`（新增命令）
- Modify: `src-tauri/src/lib.rs`（注册）

- [ ] **Step 1: 在 `commands/session.rs` 导入 `RUNNING_SESSIONS` 与 `refresh_git_info_background`**

将现有 `use crate::utils::running_sessions::{...};`（5-11 行）扩展为：

```rust
use crate::utils::running_sessions::{
    init_running_sessions,
    get_running_sessions,
    start_polling,
    stop_polling,
    RunningSession,
    RUNNING_SESSIONS,
    refresh_git_info_background,
};
```

- [ ] **Step 2: 在 `commands/session.rs` 新增命令（放在 `list_running` 之后）**

```rust
/// 手动刷新所有运行中 session 的 git 信息（后台非阻塞，force 采集）。
#[tauri::command]
pub fn refresh_git_info_all(app_handle: tauri::AppHandle) -> Result<(), String> {
    info!("[refresh_git_info_all] 开始：对所有运行中 session 触发 git 信息采集");
    let pids: Vec<u32> = {
        let sessions = RUNNING_SESSIONS.lock().unwrap();
        sessions.keys().cloned().collect()
    };
    let count = pids.len();
    for pid in pids {
        refresh_git_info_background(pid, app_handle.clone(), true);
    }
    info!("[refresh_git_info_all] 已派发 {} 个 session 的采集任务", count);
    Ok(())
}
```

- [ ] **Step 3: 在 `lib.rs` 导入新命令**

在 `use commands::session::{...};`（5-18 行）的导入列表中，`delete_session_cmd,` 之后加入：

```rust
    refresh_git_info_all,
```

- [ ] **Step 4: 在 `lib.rs` 的 `invoke_handler` 注册新命令**

在 `delete_session_cmd,`（136 行）之后加入：

```rust
            refresh_git_info_all,
```

- [ ] **Step 5: 编译验证**

Run: `cd src-tauri && cargo build`
Expected: 编译成功。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/session.rs src-tauri/src/lib.rs
git commit -m "feat: 新增 refresh_git_info_all 命令"
```

---

### Task 8: 前端 `GitInfo` 类型

**Files:**
- Modify: `src/types/session.ts:29-41`

- [ ] **Step 1: 在 `RunningSession` 接口之前新增 `GitInfo` 接口**

在 `// Running session (from Tauri backend)` 注释之前插入：

```ts
// 工作目录的 git 概要信息（snake_case，与后端 RunningSession 一致）
export interface GitInfo {
  branch: string
  is_detached: boolean
  is_worktree: boolean
  ahead: number
  behind: number
  dirty: boolean
  last_commit_sha: string
  last_commit_message: string
}
```

- [ ] **Step 2: 给 `RunningSession` 接口加 `git_info` 字段**

在 `RunningSession` 接口中，`custom_name?: string` 之后追加：

```ts
  git_info?: GitInfo
```

- [ ] **Step 3: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误。

- [ ] **Step 4: Commit**

```bash
git add src/types/session.ts
git commit -m "feat: 前端新增 GitInfo 类型与 RunningSession.git_info"
```

---

### Task 9: `SessionCardNew` 新增 git 信息行

**Files:**
- Modify: `src/components/running/SessionCard.tsx:7`（导入图标）
- Modify: `src/components/running/SessionCard.tsx:159-175`（新增 git 行）

- [ ] **Step 1: 导入 `GitBranch` 图标**

将 `import { Star, Clock } from "lucide-react"`（7 行）改为：

```ts
import { Star, Clock, GitBranch } from "lucide-react"
```

- [ ] **Step 2: 在"元信息行"之后新增 git 行**

在 `SessionCardNew` 的元信息行 `</div>`（约 174 行，`<span>Session ID: ...</span>` 所在 `div` 的闭合）之后、组件外层 `<div className="flex-1 min-w-0">` 闭合之前，插入：

```tsx
        {/* git 信息行 */}
        {session.git_info && (
          <div className="text-xs text-gray-500 mt-1 flex flex-wrap items-center gap-x-2">
            <span className="flex items-center gap-1">
              <GitBranch className="w-3 h-3 text-gray-400" />
              <span className="text-gray-700">{session.git_info.branch}</span>
            </span>
            {session.git_info.dirty && (
              <span className="text-red-500" title="有未提交更改">●</span>
            )}
            {session.git_info.is_worktree && (
              <span className="text-violet-500" title="位于 git worktree">worktree</span>
            )}
            {!compact && (session.git_info.ahead > 0 || session.git_info.behind > 0) && (
              <span title={`领先 ${session.git_info.ahead} / 落后 ${session.git_info.behind}`}>
                ↑{session.git_info.ahead} ↓{session.git_info.behind}
              </span>
            )}
            {!compact && session.git_info.last_commit_sha && (
              <span
                className="truncate"
                title={session.git_info.last_commit_message}
              >
                最近提交: {session.git_info.last_commit_sha} {session.git_info.last_commit_message}
              </span>
            )}
          </div>
        )}
```

- [ ] **Step 3: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误。

- [ ] **Step 4: Commit**

```bash
git add src/components/running/SessionCard.tsx
git commit -m "feat: SessionCardNew 显示 git 信息行"
```

---

### Task 10: 刷新按钮触发 `refresh_git_info_all`

**Files:**
- Modify: `src/components/running/RunningTab.tsx:1-12`（导入 invoke）
- Modify: `src/components/running/RunningTab.tsx:47-51`（handleRefresh）

- [ ] **Step 1: 导入 `invoke`**

在 `import { jumpToTerminal } from "@/services"`（10 行）之后追加：

```ts
import { invoke } from "@tauri-apps/api/core"
```

- [ ] **Step 2: 在 `handleRefresh` 中触发 git 全量刷新**

将 `handleRefresh`（47-51 行）替换为：

```ts
  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      // 派发后台全量 git 信息采集（force，绕过去重）
      await invoke('refresh_git_info_all')
    } catch (e) {
      // git 采集失败不阻塞主刷新流程
      console.warn('refresh_git_info_all 失败', e)
    }
    await refresh()
    setRefreshing(false)
  }
```

- [ ] **Step 3: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误。

- [ ] **Step 4: Commit**

```bash
git add src/components/running/RunningTab.tsx
git commit -m "feat: 运行中页刷新按钮触发 git 信息全量采集"
```

---

### Task 11: 端到端验证

**Files:** 无（仅运行）

- [ ] **Step 1: Rust 全量测试**

Run: `cd src-tauri && cargo test --lib`
Expected: 全部 PASS。

- [ ] **Step 2: 前端类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误。

- [ ] **Step 3: 前端构建**

Run: `npm run build`
Expected: 构建成功。

- [ ] **Step 4: 手动验证（开发者运行）**

Run: `$env:RUST_LOG = "claude_fleet=debug"; npm run tauri dev`

验证步骤：
1. 在一个 git 仓库目录启动一个 Claude Code session，确认"运行中"卡片出现 git 行，显示正确分支名。
2. 在该目录 `git checkout` 切到另一分支，等待 session 进入等待输入态（或点刷新），确认分支名更新。
3. 在该目录新建未提交文件，等待/刷新，确认出现红色 `●`。
4. 在一个非 git 目录启动 session，确认卡片不显示 git 行。
5. 若使用 worktree：在 worktree 目录启动 session，确认显示 `worktree` 标记。

Expected: 各场景表现符合预期；日志中可见 `[gather_git_info]` 与 `[refresh_git_info_background]` 记录。

- [ ] **Step 5: 最终 Commit（如有验证中发现的修复）**

```bash
git add -A
git commit -m "test: 运行中页面 git 信息功能端到端验证通过"
```

> 若验证无修复则跳过此步。

---

## 自检清单

- **Spec 覆盖**：
  - 复用 `execute_git`/`get_dirty_file_count`/worktree 检测 → Task 1、3 ✅
  - 抽取 `is_worktree` 小重构 → Task 1 ✅
  - `GitInfo` + `gather_git_info`（snake_case）→ Task 3 ✅
  - 三个小工具函数 → Task 2 ✅
  - `RunningSession.git_info` 字段 → Task 4 ✅
  - 后台非阻塞采集 + 5s 去重 + force → Task 5 ✅
  - 首次加入触发 → Task 6 Step 2 ✅
  - 转入等待态触发（复用 watcher 现有条件，不改 `update_session_status_from_file` 签名）→ Task 6 Step 3 ✅
  - 手动刷新命令 + force → Task 7 ✅
  - 前端类型 → Task 8 ✅
  - 卡片 git 行（精简/详细）→ Task 9 ✅
  - 刷新按钮触发 → Task 10 ✅
  - 错误处理（非仓库 None、无上游 0/0、采集失败 warn）→ Task 3/5 ✅
  - 测试（is_worktree、三工具、gather_git_info、truncate）→ Task 1/2/3 ✅
- **占位符扫描**：无 TBD/TODO，每步含完整代码。
- **类型一致性**：`GitInfo` 字段名前后端一致（branch/is_detached/is_worktree/ahead/behind/dirty/last_commit_sha/last_commit_message）；`refresh_git_info_background(pid, app_handle, force)` 在 Task 5/6/7 调用签名一致；`refresh_git_info_all` 命令名在 Task 7/10 一致。
