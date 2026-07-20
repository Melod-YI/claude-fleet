# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Claude Fleet 是一个 Tauri 2.0 桌面应用（仅支持 Windows），用于管理多个 Claude Code session。核心功能：
- 实时监控运行中的 session 状态（busy/idle/waiting）
- 一键跳转到对应的终端窗口（WezTerm、cmd、PowerShell、PowerShell 7）
- 历史 session 管理（收藏、搜索、恢复）
- Git worktree 管理（创建/列表/删除 worktree，跟踪仓库，删除前安全预检）

前端为 3 个 Tab：运行中（running）、worktree、Session 管理（management）。

UI 语言为中文（Simplified Chinese）。

## 开发命令

```bash
# 安装依赖
npm install

# 开发模式（同时启动前端 Vite 和 Tauri Rust 后端）
npm run tauri dev

# 构建发布版本（前端 TS 编译 + Rust 编译 + NSIS 安装包）
npm run tauri build

# 仅前端开发（Vite dev server，端口 5173）
npm run dev

# 仅前端构建（tsc + vite build）
npm run build

# Rust 单元测试
cd src-tauri && cargo test

# 前端类型检查（无独立 lint 命令，通过 tsc 检查）
npx tsc --noEmit
```

### 开启 DEBUG 日志

```bash
# 仅本项目 DEBUG（推荐）
$env:RUST_LOG = "claude_fleet=debug"; npm run tauri dev

# 全量 DEBUG
$env:RUST_LOG = "debug"; npm run tauri dev
```

日志文件：`%USERPROFILE%\.claude-fleet\logs\claude-fleet-YYYY-MM-DD.log`，保留 7 天。日志双输出：stdout（彩色精简）+ 文件（完整无颜色）。

## 架构概览

### 数据流

```
Claude Code 写入文件                   Tauri 后端                        前端
─────────────────                    ──────────                       ──────
~/.claude/projects/<proj>/<sid>.jsonl ─→ claude_session.rs 解析 JSONL ─→ invoke 返回 SessionMeta
~/.claude/sessions/<pid>.json        ─→ sessions_watcher (notify)  ─→ emit("running_sessions_changed")
                                    ─→ running_sessions.rs 状态管理 ─→ emit("session_waiting_input")
```

双轮询策略：文件系统监听器（`notify` crate）提供实时更新 + 定期轮询（30 秒间隔，`start_polling_cmd`）作为兜底检测崩溃 session。

### 前端（React + TypeScript）

**路径别名**：`@/*` → `src/*`（tsconfig.json + vite.config.ts 中配置）

**类型定义** - `src/types/`
- `session.ts`: SessionMeta, SessionMessage, RunningSession, LaunchSettings, LaunchMode 等核心类型
- `settings.ts`: TerminalType, CommandWrapperSettings, AppSettings 等设置类型
- `conversation.ts`: Conversation 类型
- `worktree.ts`: worktree 相关类型（WorktreeListItem, RepoInfo, DeletionSafety 等，与后端 camelCase 对齐）
- `tauri.d.ts`: Tauri 相关 TS 声明
- `index.ts`: 统一导出

**状态管理** - `src/stores/` (Zustand)
- `favoriteStore.ts`: 收藏列表
- `settingsStore.ts`: 应用设置（终端类型、通知、主题、启动配置）
- `sessionStore.ts`: session 状态

**Hooks** - `src/hooks/`
- `useRunningSessions.ts`: 运行中 session 状态轮询 + 事件监听
- `useSessionSearch.ts`: session 搜索逻辑
- `useNotification.ts`: Web Notifications API 封装
- `useSessions.ts`: session 数据获取

**服务层** - `src/services/` 封装 Tauri invoke 调用
- `claudeSession.ts`: session 数据操作（旧版，部分功能被 lib/api 取代）
- `terminalService.ts`: 终端窗口跳转
- `dbService.ts`: SQLite 数据操作（收藏、设置、路径、迁移）
- `sessionLaunchService.ts`: session 启动/恢复
- `notificationService.ts`: 通知编排
- `soundService.ts`: 音频播放

**API 层** - `src/lib/api/` TanStack Query 实际使用的 invoke 封装
- `sessions.ts`: session 相关的 invoke 调用（queries.ts 从此导入，而非 services/）
- `worktrees.ts`: worktree 相关的 invoke 调用

**组件** - `src/components/`
- `running/`: Running Tab（RunningTab, SessionCard, StatusBadge）
- `management/`: Session 管理 Tab（ManagementTab, GroupedSessionList, WorkspaceGroupItem, SessionList, SessionListItem, SessionDetail, ConversationView, DirectoryTree, SearchBar, TimeRangeSelect）
- `worktree/`: Worktree Tab（WorktreeTab, WorktreeDetail, RepoTree, RepoTreeItem, WorktreeTreeItem, CreateWorktreeDialog, DeleteWorktreeDialog）
- `dialogs/`: 对话框（NewSessionDialog, SettingsDialog, ConfirmDialog, ErrorDialog, PathCard）
- `layout/`: 布局组件（AppLayout, TabHeader, SplitPane）
- `common/`: 通用组件（EditableName, ErrorBoundary, PathHoverDisplay, Toggle）
- `ui/`: shadcn/ui 基础组件（violet 主题色）

**数据请求** - `src/lib/query/` (TanStack Query)
- `queries.ts`: useSessionsQuery, useSessionMessagesQuery（staleTime 覆盖为 30s）
- `worktreeQueries.ts`: worktree 列表/仓库信息/计数等 query
- `mutations.ts`: 变更定义
- `worktreeMutations.ts`: worktree 创建/删除/跟踪仓库等 mutation
- `queryClient.ts`: 默认 retry=1, staleTime=0, refetchOnWindowFocus=true, mutations.retry=false

### 后端（Rust + Tauri）

**命令** - `src-tauri/src/commands/`（5 个文件，32 个命令；db/ 下另有 18 个命令，共 50 个注册到 `invoke_handler`）
- `session.rs`（13 个）：session 生命周期（init_running, list_running, refresh_git_info_all, start/stop_polling_cmd, get_conversation, refresh_sessions, delete_session_cmd, start_new_session, start/stop_sessions_watcher, start/stop_hooks）
- `session_commands.rs`（3 个）：优化版管理命令（list_sessions_optimized, get_session_messages_optimized, delete_session_optimized）
- `terminal.rs`（7 个）：终端跳转（jump_to_terminal, jump_to_terminal_by_pid, smart_jump_to_terminal, resume_in_terminal, launch_session, open_directory, open_in_vscode）
- `sound.rs`（2 个）：get_available_sounds, get_sound_data（嵌入 + 外部文件）
- `worktree.rs`（7 个）：worktree 管理（create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd, fetch_repo_remotes_cmd, delete_worktree_cmd, preflight_delete_worktree_cmd, count_worktrees_cmd）

db/ 下的命令（同样注册为 Tauri 命令）：favorites(4)、favorite_paths(4)、sessions_meta(3)、settings(3)、tracked_repos(3)、migration(1)。

**数据库** - `src-tauri/src/db/` (SQLite via rusqlite bundled)
- 数据库位置：`~/.claude-fleet/data/claude-fleet.db`
- `schema.rs`: 6 张表 - favorites, app_settings, sessions_meta, favorite_paths, worktrees, tracked_repos（`init_tables()` 用 `CREATE TABLE IF NOT EXISTS`，并对 `favorite_paths` 做 `pinned`/`pinned_at` 列迁移）
- `sessions_meta.rs`: 自定义名称 CRUD
- `favorites.rs`: 收藏 session
- `favorite_paths.rs`: 常用路径（使用计数 + 置顶 + 加权排序：60% 时间衰减 + 40% 频率，30 天窗口）
- `settings.rs`: 键值对设置存储
- `worktrees.rs`: worktree 记录 CRUD（`WorktreeInfo`，path UNIQUE）
- `tracked_repos.rs`: 跟踪的仓库列表 CRUD（`TrackedRepo`，path UNIQUE）
- `migration.rs`: localStorage → SQLite 一次性迁移（前端 App.tsx 首次加载时触发）

**工具** - `src-tauri/src/utils/`
- `claude_data.rs`: 读取 `~/.claude/projects/` 目录，解析 JSONL 文件（旧版）
- `claude_session.rs`: session 扫描优化（head+tail 读取 JSONL）
- `running_sessions.rs`: 运行中 session 状态管理、SessionFileContent、HookEvent；away summary 扫描带 60 秒缓存；`check_processes_parallel()` 使用线程池并行检测 PID
- `sessions_watcher.rs`: `notify` crate 文件系统监听
- `window_manager.rs`: Windows API 窗口管理（HWND 缓存 + PID 链追踪 + 并行窗口标题获取）
- `logger.rs`: tracing 日志配置（双输出层：stdout 彩色 + 文件全量，7 天保留）
- `session_types.rs`: SessionMeta, SessionMessage（`#[serde(rename_all = "camelCase")]`）
- `session_utils.rs`: session 解析辅助函数
- `process.rs`: `process::command()` 封装（Windows 上自动加 `CREATE_NO_WINDOW`，见"终端集成"）
- `launch/mod.rs`: LaunchSettings, LaunchRequest, LaunchMode, SpawnPlan, CommandWrapper, build_agent_argv()
- `git/`: git 命令封装层（`mod.rs` 通用命令 `execute_git`/远端/分支/ahead-behind/dirty count 等、`info.rs` 仓库信息、`worktree.rs` worktree 创建/删除/实时列表）

## 关键类型映射

前后端序列化对齐方式**因类型而异**：

| 后端 (Rust) | 前端 (TypeScript) | Serde 策略 | 文件 |
|---|---|---|---|
| SessionMeta | SessionMeta | camelCase | session_types.rs / session.ts |
| SessionMessage | SessionMessage | camelCase | session_types.rs / session.ts |
| RunningSession | RunningSession | **snake_case（无 rename）** | running_sessions.rs / session.ts |
| LaunchSettings | LaunchSettings | camelCase | launch/mod.rs / session.ts (settings.ts 重导出) |
| CommandWrapper | CommandWrapperSettings | camelCase | launch/mod.rs / settings.ts |
| LaunchMode | LaunchMode | camelCase (tagged enum) | launch/mod.rs / session.ts |
| WorktreeInfo | — (仅后端) | camelCase | db/worktrees.rs |
| WorktreeListItem / RepoInfo / DeletionSafety / FetchResult | 对应 TS 类型 | camelCase | commands/worktree.rs / worktree.ts |
| WorktreeStatus | — | lowercase (枚举值 `"active"`/`"missing"`/`"unmanaged"`) | commands/worktree.rs |
| TrackedRepo | 对应 TS 类型 | camelCase | db/tracked_repos.rs |
| SessionFileContent | — (仅后端) | — | running_sessions.rs |
| HookEvent | — (事件 payload) | — | running_sessions.rs |

**注意**：`RunningSession` 不使用 `rename_all`，前后端均以 snake_case 字段通信（`session_id`, `updated_at`, `away_summary` 等）。

## 终端集成

终端跳转仅支持 Windows（`#[cfg(target_os = "windows")]`），非 Windows 返回错误。

支持 4 种终端类型：

| 终端 | 命令 | 创建标志 |
|---|---|---|
| cmd | `cmd.exe /K "{command_line}"` + `current_dir(cwd)` | `CREATE_NEW_CONSOLE` (0x10) |
| powershell | `powershell.exe -Command "{command_line}"` + `current_dir(cwd)` | `CREATE_NEW_CONSOLE` (0x10) |
| powershell7 | `pwsh.exe -Command "{command_line}"` + `current_dir(cwd)` | `CREATE_NEW_CONSOLE` (0x10) |
| wezterm | `wezterm.exe start --cwd {cwd} -e {process_argv}` | `DETACHED_PROCESS` (0x08) |

Windows Terminal 不作为独立终端类型：用户若想用 WT，在 Windows 系统设置中将"默认终端应用程序"设为 Windows Terminal，启动 cmd/powershell 即由 Windows 路由到 WT。

**重要**：`start` 命令会短暂显示窗口，**不要使用**。

**Windows Terminal 跳转**：WT 单进程持多 tab，父链查找无法定位 tab。`utils/window_manager.rs` 的 `find_window_by_console_attach` 用 `AttachConsole(pid)+GetConsoleWindow()` 拿到 per-tab 的 pseudo-console 宿主窗口（不可见，owner 进程名为 `WindowsTerminal.exe` 时才采用，否则回退父链，保证 cmd/ps/wezterm/git-bash 行为不变），`activate_console_window` 对该 pseudo HWND 调 `SetForegroundWindow`，WT v1.14+ 传播到主窗口并切到正确 tab。attach 序列操作进程级 console 状态，由 `CONSOLE_ATTACH_MUTEX` 串行化。

**后台命令**（git、tasklist、wmic、code 等）统一使用 `crate::utils::process::command()` 创建，自动在 Windows 上添加 `CREATE_NO_WINDOW` (0x08000000) 标志：

```rust
// ✅ 正确：使用 process::command()
use crate::utils::process;
let output = process::command("tasklist").args(["/FI", "PID eq 1234"]).output()?;

// ❌ 错误：直接用 Command::new()（会弹出控制台窗口）
let output = Command::new("tasklist").args(["/FI", "PID eq 1234"]).output()?;
```

**例外**：需要窗口可见的场景（如启动终端 `launch/mod.rs`）直接用 `Command::new()` 配合 `CREATE_NEW_CONSOLE` 或 `DETACHED_PROCESS`。

**CommandWrapper 限制**：当 `terminal_id == "wezterm"` 时，`build_process_argv()` 会强制跳过 wrapper（如 ccglass），因兼容性问题。

Launch 系统（`utils/launch/mod.rs`）支持可配置的启动参数，包括终端类型、Claude 可执行文件路径、额外参数、命令包装器、启动后窗口最大化（`maximize_window`：后台轮询定位 spawn 出的终端进程窗口并 `ShowWindow(SW_MAXIMIZE)`，cmd/ps/ps7/wezterm 通用，详见 `window_manager.rs::maximize_terminal_window`）。

## Git Worktree 管理

Worktree Tab 提供图形化 git worktree 管理，数据来源是 **git 实时状态 + SQLite 持久化记录的融合**。

- `commands/worktree.rs`：7 个命令，负责融合 git（`utils/git/worktree.rs` 的 `list_worktrees_live`）与 DB（`db/worktrees.rs`）数据
- 列表项状态：`Active`（DB+git 都有）/`Missing`（DB 有但 git 已无）/`Unmanaged`（git 有但 DB 未托管，如手动 `git worktree add` 的）
- `tracked_repos` 表：用户跟踪的仓库列表，作为 Worktree Tab 的入口
- 删除安全预检（`preflight_delete_worktree_cmd`）：未提交变更 > 0 阻断；`will_delete_branch` 且相对 base_ref 有未合并提交则阻断。base_ref 优先用创建时记录值（须可被 `git rev-parse` 解析），失效时回退仓库默认分支
- `fetch_repo_remotes_cmd`：`git fetch --all --prune`（30s 超时），失败/超时返回 `Ok(FetchResult { success: false })` 供前端降级显示本地缓存分支
- 删除 worktree 必须从**主仓库根目录**执行 git 命令（否则 cwd 落在被删目录内会 Permission denied）——主仓库路径优先取 DB 记录，未托管时回退 `get_main_repo_root()`

## 音频资源嵌入（便携版支持）

音频通过 `include_bytes!` 编译时嵌入 exe，实现免安装便携版。

- `build.rs` 扫描 `src-tauri/sounds/` 目录，生成 `embedded_sounds.rs`
- `include_bytes!` 路径相对于 OUT_DIR（`target/release/build/<hash>/out/`），需 `../../../sounds/` 回退
- `#[cfg(not(debug_assertions))]` 区分开发模式（读文件）和生产模式（读嵌入数据）
- 同时支持外部 `sounds/` 目录用于用户自定义音频

## CI/CD

`.github/workflows/release.yml`：在 `v*.*.*` tag 或手动 dispatch 时触发，在 `windows-latest` 上构建，产出便携版 exe 并发布到 GitHub Releases。

## 开发规范

### 命名约定

| 元素 | 约定 | 示例 |
|---|---|---|
| 组件文件 | PascalCase | `SessionCard.tsx` |
| 服务/hook 文件 | camelCase | `terminalService.ts` |
| Tauri 命令 (Rust) | snake_case | `jump_to_terminal` |
| 前端 invoke 调用 | camelCase | `jumpToTerminal` |
| Rust 结构体 | PascalCase | `SessionMeta` |
| Serde 序列化 | 视类型而定 | SessionMeta 用 camelCase，RunningSession 保持 snake_case |
| CSS 类名 | Tailwind utility + cn() | `cn("base", cond && "active")` |

### 日志规范

后端使用 `tracing` 宏，日志格式：`[method_name] 描述: 参数`

- `info!`: 正常业务流程、重要操作结果
- `debug!`: 详细执行过程、中间状态
- `warn!`: 非预期但可恢复的情况
- `error!`: 错误、异常、操作失败

必须添加日志的位置：方法入口/出口、核心分支决策点、错误捕获、状态变化。

### Windows 特定代码

- 使用 `#[cfg(target_os = "windows")]` 条件编译
- 非 Windows 平台返回 `"仅支持 Windows 平台"` 错误
- 运行中 session 检测通过 tasklist 命令检查进程 PID 是否存在
