# Git Worktree 后端功能设计 — 第一期（创建 + 列表）

## 概述

为 Claude Fleet 添加 git worktree 管理能力。第一期聚焦后端两项核心功能：

1. **创建 worktree** — 在指定仓库下创建 worktree，持久化到 SQLite，并复制 `.claude` 配置
2. **列表 worktree** — 查询指定仓库下所有 worktree，融合数据库记录与 git 实时状态

## 架构

### 新增文件

| 文件 | 职责 |
|---|---|
| `src-tauri/src/utils/git/mod.rs` | 通用 git 命令封装层 |
| `src-tauri/src/utils/git/worktree.rs` | worktree 业务逻辑（创建、列表） |
| `src-tauri/src/db/worktrees.rs` | worktrees 表 SQLite CRUD |
| `src-tauri/src/commands/worktree.rs` | Tauri invoke 命令 |

### 修改文件

| 文件 | 变更 |
|---|---|
| `src-tauri/src/utils/mod.rs` | 添加 `pub mod git;` |
| `src-tauri/src/db/schema.rs` | `init_tables()` 添加 worktrees 建表语句 |
| `src-tauri/src/db/mod.rs` | 添加 `pub mod worktrees;` |
| `src-tauri/src/commands/mod.rs` | 添加 `pub mod worktree;` |
| `src-tauri/src/lib.rs` | 注册新 Tauri 命令 |

### 数据流

```
前端                          Tauri 命令                    业务逻辑                    Git
────                         ──────────                   ────────                    ────
invoke("createWorktree",   → commands/worktree.rs        → git/worktree.rs          → git -C <path> ...
     repoPath, name,         create_worktree_cmd()         create_worktree()           execute_git()
     branch, baseRef)                                      (验证→创建分支→添加worktree
                                                           →复制.claude→返回结果)
                                                         → db/worktrees.rs
                                                           insert_worktree()

invoke("listWorktrees",    → commands/worktree.rs        → git/worktree.rs          → git -C <path>
     repoPath)               list_worktrees_cmd()          list_worktrees_live()       worktree list --porcelain
                                                         → db/worktrees.rs
                                                           list_worktrees_by_repo()

invoke("getRepoInfo",      → commands/worktree.rs        → git/mod.rs               → git -C <path>
     repoPath)               get_repo_info_cmd()           get_remotes()               remote -v
                                                           get_local_branches()         branch --list
                                                           get_remote_branches()        branch -r
                                                           get_default_branch()         symbolic-ref
```

---

## Git 工具层 (`utils/git/mod.rs`)

### 核心执行器

```rust
/// 在指定仓库目录执行 git 命令。
/// 通过 `git -C <repo_path>` 执行，无需改变进程目录。
/// 成功返回 stdout（trim），失败返回包含 stderr 的错误信息。
pub fn execute_git(repo_path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .map_err(|e| format!("无法执行 git 命令: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("git 命令失败: {}", stderr))
    }
}
```

### 仓库信息函数

```rust
/// 获取仓库名称。优先从 remote URL 提取，回退到目录名。
pub fn get_repo_name(repo_path: &Path) -> Result<String, String>

/// 获取远程仓库列表。
pub fn get_remotes(repo_path: &Path) -> Result<Vec<RemoteInfo>, String>

/// 获取本地分支列表（解析 `git branch --list --format=%(refname:short)`）。
pub fn get_local_branches(repo_path: &Path) -> Result<Vec<String>, String>

/// 获取远程分支列表（解析 `git branch -r --format=%(refname:short)`）。
pub fn get_remote_branches(repo_path: &Path) -> Result<Vec<String>, String>

/// 检测默认分支。优先级：
/// 1. `git symbolic-ref refs/remotes/origin/HEAD`
/// 2. 检查常见分支名是否存在（main, master, develop）
/// 3. 回退 "main"
pub fn get_default_branch(repo_path: &Path) -> Result<String, String>

/// 检查本地分支是否存在（`git show-ref --verify --quiet refs/heads/<branch>`）。
pub fn branch_exists(repo_path: &Path, branch: &str) -> bool

/// 获取仓库的父目录（用于计算 worktree 目录路径）。
/// 对于 worktree 路径，使用 `git rev-parse --git-common-dir` 定位主仓库。
pub fn get_repo_parent(repo_path: &Path) -> Result<PathBuf, String>
```

### 类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub name: String,    // "origin", "upstream" 等
    pub url: String,     // 远程 URL
}
```

### URL 名称提取

```rust
/// 从远程 URL 提取仓库名称。
/// 支持: https://github.com/user/repo.git, git@github.com:user/repo.git
pub fn extract_repo_name_from_url(url: &str) -> Option<String>
```

---

## Worktree 业务逻辑 (`utils/git/worktree.rs`)

### 类型

```rust
#[derive(Debug, Clone)]
pub struct CreateWorktreeOptions {
    pub repo_path: PathBuf,       // 主仓库路径
    pub name: String,             // worktree 名称
    pub branch: String,           // 目标分支名
    pub base_ref: String,         // 基点引用（如 "origin/main"）
}

/// git worktree list --porcelain 的解析结果（仅包含 git 原始数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorktreeEntry {
    pub path: String,
    pub head: String,             // HEAD commit hash
    pub branch: Option<String>,   // 分支名（detached HEAD 时为 None）
    pub is_bare: bool,
    pub is_main: bool,            // 是否为主仓库
}
```

### 创建流程

```rust
pub fn create_worktree(opts: &CreateWorktreeOptions) -> Result<WorktreeInfo, String> {
    // 1. 验证 repo_path 是有效的 git 仓库
    //    execute_git(repo_path, &["rev-parse", "--is-inside-work-tree"])

    // 2. 获取 repo_name
    //    get_repo_name(repo_path)

    // 3. 计算目标目录
    //    parent = get_repo_parent(repo_path)
    //    worktree_base = parent.join(format!("{}.worktrees", repo_name))
    //    worktree_dir = worktree_base.join(&sanitized_name)

    // 4. 检查冲突
    //    - 目录是否已存在
    //    - git worktree list 是否已包含该路径

    // 5. 创建分支（如果不存在）
    //    if !branch_exists(repo_path, &opts.branch) {
    //        execute_git(repo_path, &["branch", &opts.branch, &opts.base_ref])
    //    }

    // 6. 确保 worktree 根目录存在
    //    fs::create_dir_all(&worktree_base)

    // 7. 创建 worktree
    //    使用相对路径: ../<repo>.worktrees/<name>
    //    execute_git(repo_path, &["worktree", "add", &relative_path, &opts.branch])

    // 8. 复制 .claude 目录（如果存在）
    //    copy_claude_dir(repo_path, &worktree_dir)

    // 9. 返回 WorktreeInfo（数据库记录结构体）
}
```

### .claude 目录复制

```rust
/// 递归复制主仓库的 .claude 目录到 worktree。
/// 使用标准库 fs 实现，无需引入额外依赖。
fn copy_claude_dir(src_repo: &Path, dst_worktree: &Path) -> Result<(), String> {
    let src = src_repo.join(".claude");
    if !src.exists() {
        return Ok(());  // 没有 .claude 目录则跳过
    }
    let dst = dst_worktree.join(".claude");
    copy_dir_recursive(&src, &dst)
}

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
```

### 目录名 Sanitize

```rust
/// 清理名称用于目录名：替换 Windows 非法字符为 `-`，去除首尾空格和点。
pub fn sanitize_name(name: &str) -> String {
    let mut result = name.trim().to_string();
    // 替换 Windows 文件名非法字符
    for ch in ['<', '>', ':', '"', '|', '?', '*', '/', '\\'] {
        result = result.replace(ch, "-");
    }
    // 去除首尾的点（避免隐藏目录或相对路径问题）
    result.trim_matches('.').to_string()
}
```

### 列表（实时 git 数据）

```rust
/// 解析 `git worktree list --porcelain` 输出。
/// 返回所有 worktree 的实时状态（包括主仓库本身）。
pub fn list_worktrees_live(repo_path: &Path) -> Result<Vec<GitWorktreeEntry>, String> {
    let output = execute_git(repo_path, &["worktree", "list", "--porcelain"])?;
    parse_worktree_porcelain(&output)
}

/// 解析 porcelain 格式：
/// worktree /path/to/repo
/// HEAD abc123
/// branch refs/heads/main
///
/// worktree /path/to/worktree
/// HEAD def456
/// branch refs/heads/feature
fn parse_worktree_porcelain(output: &str) -> Result<Vec<GitWorktreeEntry>, String>
```

---

## 数据库设计 (`db/worktrees.rs`)

### 表结构

```sql
CREATE TABLE IF NOT EXISTS worktrees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    branch TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    repo_name TEXT NOT NULL,
    repo_path TEXT NOT NULL,
    base_ref TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

### Rust 类型

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
    pub id: i64,
    pub name: String,
    pub branch: String,
    pub path: String,           // 绝对路径
    pub repo_name: String,
    pub repo_path: String,      // 主仓库绝对路径
    pub base_ref: String,       // 创建时的基点引用（如 "origin/main"）
    pub created_at: i64,        // Unix 时间戳（秒）
}
```

### CRUD 函数

```rust
/// 插入 worktree 记录。path 有 UNIQUE 约束，重复插入会报错。
pub fn insert_worktree(conn: &Connection, info: &WorktreeInfo) -> Result<(), String>

/// 按主仓库路径查询所有 worktree。
pub fn list_worktrees_by_repo(conn: &Connection, repo_path: &str) -> Result<Vec<WorktreeInfo>, String>

/// 按 worktree 路径查询单条记录。
pub fn get_worktree_by_path(conn: &Connection, path: &str) -> Result<Option<WorktreeInfo>, String>

/// 按路径删除 worktree 记录（第二期删除功能使用）。
pub fn delete_worktree_by_path(conn: &Connection, path: &str) -> Result<(), String>
```

### 初始化

在 `db/schema.rs` 的 `init_tables()` 中追加建表语句：

```rust
conn.execute_batch(
    // ... existing tables ...
    "CREATE TABLE IF NOT EXISTS worktrees (
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

---

## Tauri 命令 (`commands/worktree.rs`)

### 1. 创建 Worktree

```rust
#[tauri::command]
pub fn create_worktree_cmd(
    repo_path: String,
    name: String,
    branch: String,
    base_ref: String,
) -> Result<WorktreeInfo, String> {
    // 1. 调用 worktree::create_worktree() 执行 git 操作
    // 2. 调用 db::worktrees::insert_worktree() 持久化
    // 3. 返回 WorktreeInfo（含数据库 id）
}
```

**前端调用示例：**

```typescript
const result = await invoke<WorktreeInfo>("createWorktreeCmd", {
  repoPath: "C:\\workspace\\myproject",
  name: "feature-auth",
  branch: "feature-auth",
  baseRef: "origin/main",
});
```

### 2. 列表 Worktree

```rust
#[tauri::command]
pub fn list_worktrees_cmd(
    repo_path: String,
) -> Result<Vec<WorktreeListItem>, String> {
    // 1. 从数据库查询该仓库的 worktree 记录
    // 2. 从 git worktree list --porcelain 获取实时数据
    // 3. 融合两者：数据库记录为基准，匹配实时 git 状态
    //    - 数据库有但 git 没有 → 标记为 missing
    //    - git 有但数据库没有 → 标记为 unmanaged（未托管的外部 worktree）
    // 4. 返回合并后的列表
}
```

**返回类型（融合后）：**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeStatus {
    Active,     // 数据库有记录且 git worktree 中存在
    Missing,    // 数据库有记录但 git worktree 中不存在（可能已被外部删除）
    Unmanaged,  // git worktree 中存在但数据库无记录（外部创建的）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeListItem {
    // 数据库信息（unmanaged 时为 None）
    pub id: Option<i64>,
    pub name: String,
    pub repo_name: String,
    pub base_ref: Option<String>,
    pub created_at: Option<i64>,

    // Git 实时信息
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub is_main: bool,

    // 状态标记
    pub status: WorktreeStatus,  // "active" | "missing" | "unmanaged"
}
```

**融合算法：** 以 `path` 为 key 匹配数据库记录和 git 实时数据：

```
1. 获取 git_items = list_worktrees_live(repo_path)  → Vec<GitWorktreeEntry>
2. 获取 db_items  = list_worktrees_by_repo(repo_path) → Vec<WorktreeInfo>
3. 构建 db_map: HashMap<path, WorktreeInfo>
4. 构建 git_paths: HashSet<path>
5. 遍历 git_items（跳过 is_main）：
   - 匹配到 db_map → status = Active，合并字段
   - 未匹配 → status = Unmanaged，name = 从路径提取
6. 遍历 db_items：
   - 不在 git_paths 中 → status = Missing
7. 排序：Active 优先，按 created_at 降序
```

### 3. 获取仓库信息

```rust
#[tauri::command]
pub fn get_repo_info_cmd(
    repo_path: String,
) -> Result<RepoInfo, String> {
    // 1. 获取 remotes 列表
    // 2. 获取本地分支列表
    // 3. 获取远程分支列表
    // 4. 检测默认分支
    // 5. 获取仓库名称
}
```

**返回类型：**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,                   // 仓库名称
    pub remotes: Vec<RemoteInfo>,       // 远程仓库列表
    pub local_branches: Vec<String>,    // 本地分支
    pub remote_branches: Vec<String>,   // 远程分支
    pub default_branch: String,         // 默认分支名
}
```

**前端调用示例：**

```typescript
const info = await invoke<RepoInfo>("getRepoInfoCmd", {
  repoPath: "C:\\workspace\\myproject",
});
// info.remotes: [{name: "origin", url: "..."}, {name: "upstream", url: "..."}]
// info.localBranches: ["main", "develop", "feature-x"]
// info.remoteBranches: ["origin/main", "origin/develop", "upstream/main"]
// info.defaultBranch: "main"
```

---

## 命令注册

在 `lib.rs` 的 `invoke_handler` 中添加：

```rust
use commands::worktree::{create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd};

tauri::Builder::default()
    // ... existing ...
    .invoke_handler(tauri::generate_handler![
        // ... existing commands ...
        // Worktree commands
        create_worktree_cmd,
        list_worktrees_cmd,
        get_repo_info_cmd,
    ])
```

---

## 前端类型定义（参考）

在 `src/types/worktree.ts` 中定义（与后端 camelCase 序列化对齐）：

```typescript
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

---

## 与 AgentDev 项目的关键区别

| 维度 | AgentDev | Claude Fleet（本设计） |
|---|---|---|
| **持久化** | `state.json`（JSON 文件） | SQLite（`worktrees` 表） |
| **架构** | CLI + Axum Web Server | Tauri 桌面应用 |
| **错误处理** | `anyhow::Result` | `Result<T, String>`（与 Tauri 命令一致） |
| **分支策略** | CLI 场景下要求用户在 base branch 上；Web UI 场景自动从 default branch 创建 | 前端传入明确的 base_ref，不做自动推测 |
| **远程检测** | 无显式的 remotes/branches 查询 API | 提供 `get_repo_info_cmd` 供前端构建分支选择器 |
| **`.claude` 复制** | 仅复制 `CLAUDE.local.md` | 复制整个 `.claude` 目录 |
| **Submodule** | 创建后自动更新 submodule | 第一期不处理 submodule（可后续添加） |
| **tmux/Agent 启动** | 创建后自动在 tmux 中启动 agent | 第一期不包含自动启动（复用现有 launch_session） |
| **Git 日志缓冲** | 内置 ring buffer 记录最近 100 条 git 命令 | 使用 tracing 日志，不维护内存缓冲 |
| **目录名 sanitize** | `sanitize_branch_name()`（替换空格和特殊字符） | `sanitize_name()`（替换 Windows 非法字符） |
| **Worktree 列表融合** | 仅展示 state.json 中管理的 worktree | 融合数据库记录 + git 实时数据，标记 missing/unmanaged 状态 |
| **base_ref 记录** | 不记录创建时的基点引用 | 数据库存储 `base_ref` 字段，为未来 sync 功能做准备 |
