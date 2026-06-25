# 运行中页面显示 Git 信息 — 设计文档

- 日期：2026-06-25
- 分支：running-git-info
- 状态：已确认，待写实现计划

## 1. 目标

在"运行中"页面的每个 session 卡片上额外显示该 session 工作目录（`cwd`）的 git 信息：当前分支、是否 worktree、相对上游的 ahead/behind、是否有未提交更改（dirty）、最近一次提交。非 git 仓库不显示该行。

## 2. 需求决策（已与用户确认）

1. **字段范围**：标准集 — 分支名（detached 时为短 sha）、`is_detached`、`is_worktree`、`ahead`、`behind`、`dirty`、`last_commit_sha`、`last_commit_message`。
2. **刷新时机**：session 状态转入 `idle`/`waiting`（等待输入）时刷新一次；非阻塞，不要求强实时。session 首次加入时采集一次；手动"刷新"按钮触发全量重采。
3. **显示位置**：在现有"元信息行"下方新增独立 git 行。精简模式显示分支 + dirty 标记 + worktree 标记；详细模式额外显示 ahead/behind 与最近提交。非 git 仓库不渲染该行。

## 3. 复用与重构审视

现有 `src-tauri/src/utils/git/` 模块组织良好，无需大重构。

### 3.1 直接复用（`utils/git/mod.rs`）

| 现有函数 | 复用方式 |
|---|---|
| `execute_git(repo_path, args)` | 所有 git 调用的统一执行器（已带 `CREATE_NO_WINDOW` + 日志） |
| `normalize_path` | 路径分隔符归一 |
| `get_dirty_file_count(repo_path)` | `dirty = get_dirty_file_count > 0` |
| `get_repo_parent` 内的 worktree 判定逻辑（`common_dir != git_dir`） | 抽取为 `is_worktree` 后复用 |

### 3.2 唯一小重构：抽取 `is_worktree`

`get_repo_parent` 当前内联了 worktree 判定（比较 `rev-parse --git-common-dir` 与 `--git-dir`）。将其抽为：

```rust
/// 是否处于 git worktree 中（而非主仓库）。
pub fn is_worktree(repo_path: &Path) -> bool {
    let common = execute_git(repo_path, &["rev-parse", "--git-common-dir"]);
    let git_dir = execute_git(repo_path, &["rev-parse", "--git-dir"]);
    match (common, git_dir) {
        (Ok(c), Ok(g)) => normalize_path(&c) != normalize_path(&g),
        _ => false,
    }
}
```

`get_repo_parent` 改为先调用 `is_worktree` 分支，再取对应路径，消除重复。范围小、风险低，worktree 功能与本功能同时受益。

### 3.3 不改动：`get_ahead_behind`

现有 `get_ahead_behind(repo_path, branch, base_ref)` 硬编码 `origin/` 前缀、按显式 `base_ref` 比较，是 worktree 功能专用语义（比较 worktree 分支与 base_ref 如 `origin/main`）。本功能需要的是"相对上游跟踪分支 `@{u}`"的 ahead/behind，语义不同。强行泛化会改其契约并波及 worktree 调用方，违背外科手术式改动原则。新增兄弟函数 `get_upstream_ahead_behind` 解决。

## 4. 后端实现

### 4.1 新增 `utils/git/info.rs`

挂到 `git` 模块（`mod.rs` 增加 `pub mod info;`）。

**数据结构**（snake_case，与 `RunningSession` 一致，避免其 JSON 内部大小写混杂）：

```rust
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
```

**编排函数**：

```rust
pub fn gather_git_info(cwd: &Path) -> Option<GitInfo>
```

步骤（全部经 `execute_git`）：
1. `rev-parse --is-inside-work-tree` → 非 `true` 则返回 `None`（非 git 仓库）。
2. `is_worktree(cwd)` → `is_worktree`。
3. 分支：`rev-parse --abbrev-ref HEAD`。若结果为 `HEAD` → `is_detached = true`，`branch` 改取 `rev-parse --short HEAD`。
4. dirty：`get_dirty_file_count(cwd) > 0`。
5. ahead/behind：`rev-list --left-right --count @{u}...HEAD`。命令失败（无上游）→ `0/0`，`warn!` 记录。
6. 最近提交：`log -1 --format=%h%x00%s`，按 `\0` 拆分为 sha 与 message，message 截断至 60 字符。

任一步骤异常仅 `warn!`，不中断；最终用已得字段构造 `GitInfo` 返回 `Some`。仅当步骤 1 判定非仓库时返回 `None`。方法入口/出口 `info!`，各分支 `debug!`。

### 4.2 `utils/git/mod.rs` 新增小工具

- `is_worktree(repo_path) -> bool`（见 3.2）
- `get_current_branch(repo_path) -> Result<(Option<String>, bool), String>`：返回 `(分支名, is_detached)`，供 `info.rs` 与未来复用。
- `get_last_commit(repo_path) -> Result<(String, String), String>`：返回 `(短 sha, message)`。
- `get_upstream_ahead_behind(repo_path) -> (u32, u32)`：无上游返回 `(0, 0)`。

`info.rs` 的 `gather_git_info` 优先调用这些工具函数，避免逻辑重复。

### 4.3 `RunningSession` 字段扩展（`utils/running_sessions.rs`）

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub git_info: Option<GitInfo>,
```

`add_running_session_from_file` 构造时初始化 `git_info: None`（首次采集由调用方异步触发，避免阻塞添加流程）。

### 4.4 刷新编排（`utils/running_sessions.rs`）

新增全局去重缓存：

```rust
static GIT_REFRESH_CACHE: Lazy<Mutex<HashMap<String, Instant>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));  // key: cwd，value: 上次触发时刻
```

新增函数：

```rust
pub fn refresh_git_info_background(pid: u32, app_handle: tauri::AppHandle, force: bool)
```

行为：
1. `thread::spawn`（后台，不阻塞调用方）。
2. 加锁读取该 pid 的 `cwd`（clone 退出锁）。pid 不存在则直接返回。
3. 去重：检查 `GIT_REFRESH_CACHE`，若该 cwd 在 5 秒内已被触发且 `force == false` 则跳过；否则记录当前 `Instant`。`force == true`（手动刷新）绕过去重，强制采集。
4. 调用 `git::info::gather_git_info(&cwd)`。
5. 加锁写回 `RUNNING_SESSIONS[pid].git_info`。
6. `app_handle.emit("running_sessions_changed", get_running_sessions())`，失败 `error!`。

> 说明：`Instant` 在 `thread::spawn` 闭包内取 `Instant::now()` 即可（此处不是 workflow 脚本环境，标准库 `Instant` 可用）。

### 4.5 触发点接线

**转入 idle/waiting 触发**：`handle_session_modify`（`sessions_watcher.rs`）已在内部用 `old_status`（调用前经 `get_session_status_by_pid` 读取）与 `session.status` 计算 `is_waiting_now && !was_waiting_before`（用于通知事件，见 307 行）。复用该条件——在该 `if` 块内同时调用 `refresh_git_info_background(session.pid, app_handle.clone(), false)`（自动触发，受去重约束）。**不改动 `update_session_status_from_file` 签名**，避免重复实现转换检测，保持外科手术式改动。`handle_session_modify` 已持有 `app_handle: &tauri::AppHandle`。

**首次加入触发**：`handle_session_create`（`sessions_watcher.rs:226`）在 `add_running_session_from_file` 成功后，调用 `refresh_git_info_background(session.pid, app_handle.clone(), false)`。

**手动刷新触发**：新增 Tauri 命令 `refresh_git_info_all`（`commands/session.rs`），遍历 `RUNNING_SESSIONS` 所有 pid，对每个调用 `refresh_git_info_background(pid, app_handle.clone(), true)`（`force = true`，绕过去重，确保刷新按钮强制更新）。前端"刷新"按钮调用此命令（fire-and-forget，更新经 `running_sessions_changed` 事件下发）。

### 4.6 命令注册

`lib.rs` 的 `invoke_handler` 注册 `refresh_git_info_all`。

## 5. 前端实现

### 5.1 类型（`src/types/session.ts`）

```ts
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

export interface RunningSession {
  // ... 现有字段
  git_info?: GitInfo
}
```

### 5.2 展示（`src/components/running/SessionCardNew.tsx`）

在"元信息行"（当前 160-174 行）下方新增独立 git 行，仅当 `session.git_info` 存在时渲染。

- **精简模式**（`compact === true`）：
  - `GitBranch` 图标 + 分支名（detached 显示短 sha）。
  - dirty：红点 `●`（`text-red-500`），仅在 `dirty` 为真时显示。
  - worktree：`⟳worktree` 标记（`text-violet-500`），仅在 `is_worktree` 为真时显示。
- **详细模式**（`compact === false`）：在精简内容基础上追加
  - `↑{ahead} ↓{behind}`，仅当 `ahead>0 || behind>0` 时显示。
  - `最近提交: {last_commit_sha} {last_commit_message}`（message 已截断，`title` 悬浮显示完整）。

样式与现有元信息行一致（`text-xs text-gray-500`，`flex flex-wrap items-center gap-x-2`）。图标用 `lucide-react` 的 `GitBranch`。

### 5.3 手动刷新（`src/components/running/RunningTab.tsx`）

`handleRefresh` 在现有 `refresh()`（`list_running`）之外，增加 `invoke('refresh_git_info_all')`（不 await，fire-and-forget）。即时反馈由现有 `list_running` 提供，git 更新经事件下发。

## 6. 错误处理

- git 命令失败（无 git 可执行、非仓库、仓库损坏）→ `gather_git_info` 返回 `None`（非仓库）或字段降级（无上游 → 0/0），`warn!` 记录，不影响 session 主流程。
- 后台线程 panic 不影响主流程：`thread::spawn` 闭包内捕获错误路径，关键步骤用 `Result`/`match`，不 `unwrap` 跨线程边界。
- `refresh_git_info_all` 命令始终返回 `Ok(())`，错误仅日志。

## 7. 测试

### 7.1 后端 Rust 单元测试

- `git/mod.rs`：
  - `is_worktree`：在主仓库返回 `false`，在 worktree 返回 `true`（测试用临时 git 仓库 + `git worktree add`）。
  - `get_current_branch` / `get_last_commit` / `get_upstream_ahead_behind` 各覆盖正常与无上游场景。
- `git/info.rs`：
  - `gather_git_info` 在临时仓库断言各字段；覆盖 detached、dirty、有/无上游分支场景；非 git 目录返回 `None`。

> 临时 git 仓库测试用 `tempfile` crate（若未引入则用 `std::env::temp_dir` + 唯一子目录，测试结束清理）。需 git 可执行；CI 在 windows-latest 上有 git。

> 不改动 `update_session_status_from_file` 签名，故无相关返回值测试；转入检测复用 watcher 现有逻辑。

### 7.2 前端

- `SessionCardNew` 渲染 git 行：有 `git_info` 时渲染、无时不渲染；精简/详细模式字段差异。若有现成组件测试框架则补充，否则至少 `npx tsc --noEmit` 类型检查通过。

## 8. 类型映射补充

| 后端 (Rust) | 前端 (TS) | Serde 策略 | 文件 |
|---|---|---|---|
| GitInfo | GitInfo | **snake_case（无 rename）** | git/info.rs / session.ts |

`GitInfo` 嵌于 `RunningSession`，沿用其 snake_case 约定，保持 `RunningSession` JSON 内部大小写一致。

## 9. 不做（YAGNI）

- 不做 TTL 定时刷新（已改为事件驱动：转入 idle/waiting 触发）。
- 不显示暂存/未暂存分别计数、上游分支名、最近提交相对时间（属"完整集"，超出当前范围）。
- 不改动 worktree 功能的 `get_ahead_behind` 及其调用方。
- 不为 git 信息单独新增前端事件类型；复用 `running_sessions_changed`。
