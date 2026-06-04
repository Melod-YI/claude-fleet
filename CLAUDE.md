# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Claude Fleet 是一个 Tauri 2.0 桌面应用（仅支持 Windows），用于管理多个 Claude Code session。核心功能：
- 实时监控运行中的 session 状态（busy/idle/waiting）
- 一键跳转到对应的终端窗口（WezTerm、cmd、PowerShell）
- 历史 session 管理（收藏、搜索、恢复）

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

日志文件：`%USERPROFILE%\.claude-fleet\logs\claude-fleet-YYYY-MM-DD.log`，保留 7 天。

## 架构概览

### 数据流

```
Claude Code 写入文件                   Tauri 后端                        前端
─────────────────                    ──────────                       ──────
~/.claude/projects/<proj>/<sid>.jsonl ─→ claude_data.rs 解析 JSONL ─→ invoke 返回 SessionMeta
~/.claude/sessions/<pid>.json        ─→ sessions_watcher (notify)  ─→ emit("running_sessions_changed")
                                    ─→ running_sessions.rs 状态管理 ─→ emit("session_waiting_input")
```

双轮询策略：文件系统监听器（`notify` crate）提供实时更新 + 定期轮询（`start_polling_cmd`）作为兜底检测崩溃 session。

### 前端（React + TypeScript）

**路径别名**：`@/*` → `src/*`（tsconfig.json + vite.config.ts 中配置）

**状态管理** - `src/stores/` (Zustand)
- `favoriteStore.ts`: 收藏列表
- `settingsStore.ts`: 应用设置（终端类型、通知、主题、启动配置）
- `sessionStore.ts`: session 状态

**Hooks** - `src/hooks/`
- `useRunningSessions.ts`: 运行中 session 状态轮询
- `useSessionSearch.ts`: session 搜索逻辑
- `useNotification.ts`: Web Notifications API 封装
- `useSessions.ts`: session 数据获取

**服务层** - `src/services/` 封装 Tauri invoke 调用
- `claudeSession.ts`: session 数据操作
- `terminalService.ts`: 终端窗口跳转
- `dbService.ts`: SQLite 数据操作（收藏、设置、路径、迁移）
- `sessionLaunchService.ts`: session 启动/恢复
- `notificationService.ts`: 通知编排
- `soundService.ts`: 音频播放

**组件** - `src/components/`
- `running/`: Running Tab（SessionCard, StatusBadge）
- `management/`: Session 管理 Tab（ManagementTab, SessionList, SessionDetail, ConversationView, DirectoryTree, SearchBar）
- `dialogs/`: 对话框（NewSessionDialog, SettingsDialog, ConfirmDialog, ErrorDialog）
- `layout/`: 布局组件（AppLayout, TabHeader, SplitPane）
- `common/`: 通用组件（EditableName, ErrorBoundary, PathHoverDisplay, Toggle）
- `ui/`: shadcn/ui 基础组件（violet 主题色）

**数据请求** - `src/lib/query/` (TanStack Query)
- `queries.ts`: useSessionsQuery, useSessionMessagesQuery
- `mutations.ts`: 变更定义
- `queryClient.ts`: retry=1, staleTime=0, refetchOnWindowFocus=true

### 后端（Rust + Tauri）

**命令** - `src-tauri/src/commands/`（共 35 个 Tauri 命令）
- `session.rs`: session 生命周期（init_running, list_running, start/stop_polling, start/stop_sessions_watcher, start/stop_hooks）
- `session_commands.rs`: 优化版管理命令（list_sessions_optimized, get_session_messages_optimized, delete_session_optimized）
- `terminal.rs`: 终端跳转（jump_to_terminal, smart_jump_to_terminal, resume_in_terminal, launch_session, open_directory, open_in_vscode）
- `sound.rs`: 音频读取（嵌入 + 外部文件）

**数据库** - `src-tauri/src/db/` (SQLite via rusqlite bundled)
- 数据库位置：`~/.claude-fleet/data/claude-fleet.db`
- `schema.rs`: 4 张表 - favorites, app_settings, sessions_meta, favorite_paths
- `sessions_meta.rs`: 自定义名称 CRUD
- `favorites.rs`: 收藏 session
- `favorite_paths.rs`: 常用路径（使用计数 + 置顶）
- `settings.rs`: 键值对设置存储
- `migration.rs`: localStorage → SQLite 一次性迁移（前端 App.tsx 首次加载时触发）

**工具** - `src-tauri/src/utils/`
- `claude_data.rs`: 读取 `~/.claude/projects/` 目录，解析 JSONL 文件
- `claude_session.rs`: session 扫描优化
- `running_sessions.rs`: 运行中 session 状态管理、SessionFileContent
- `sessions_watcher.rs`: `notify` crate 文件系统监听
- `window_manager.rs`: Windows API 窗口管理（PID 链追踪、进程匹配）
- `logger.rs`: tracing 日志配置（日志轮转、7 天保留）
- `session_types.rs`: SessionMeta, SessionMessage（`#[serde(rename_all = "camelCase")]`）
- `session_utils.rs`: session 解析辅助函数
- `launch/mod.rs`: LaunchSettings, LaunchRequest, SpawnPlan, build_agent_argv()

## 关键类型映射

前后端通过 serde `camelCase` 序列化对齐：

| 后端 (Rust) | 前端 (TypeScript) | 文件 |
|---|---|---|
| SessionMeta | SessionMeta | session_types.rs / session.ts |
| SessionMessage | SessionMessage | session_types.rs / session.ts |
| RunningSession | RunningSession | running_sessions.rs / session.ts |
| LaunchSettings | LaunchSettings | launch/mod.rs / settings.ts |

## 终端集成

终端跳转仅支持 Windows（`#[cfg(target_os = "windows")]`），非 Windows 返回错误。

**场景一：启动终端窗口**（新建/恢复 session）
直接运行目标程序，使用 `DETACHED_PROCESS` 脱离父进程：
```rust
use std::os::windows::process::CommandExt;
const DETACHED_PROCESS: u32 = 0x00000008;
// cmd: cmd.exe /d {cwd} /K claude
// powershell: powershell.exe -NoExit -Command "cd '{cwd}'; claude"
// wezterm: wezterm start --cwd {cwd} -e claude
```

**场景二：后台命令**（tasklist、wmic、code 等）
通过 `cmd.exe /C` + `CREATE_NO_WINDOW` 隐藏窗口：
```rust
const CREATE_NO_WINDOW: u32 = 0x08000000;
Command::new("cmd.exe").args(["/C", "code", &path]).creation_flags(CREATE_NO_WINDOW).spawn()
```

**要点**：`start` 命令会短暂显示窗口，**不要使用**。需引入 `CommandExt` trait。

Launch 系统（`utils/launch/mod.rs`）支持可配置的启动参数，包括终端类型、Claude 可执行文件路径、额外参数、命令包装器（如 ccglass）。

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
| Serde 序列化 | camelCase | `#[serde(rename_all = "camelCase")]` |
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
