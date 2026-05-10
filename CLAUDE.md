# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Claude Fleet 是一个 Tauri 2.0 桌面应用，用于管理多个 Claude Code session。核心功能：
- 实时监控运行中的 session 状态（busy/idle/waiting）
- 一键跳转到对应的终端窗口（WezTerm、cmd、PowerShell）
- 历史 session 管理（收藏、搜索、恢复）

## 开发命令

```bash
# 安装依赖
npm install

# 开发模式（同时启动前端和 Tauri）
npm run tauri dev

# 构建发布版本
npm run tauri build

# 仅前端开发
npm run dev

# 仅前端构建
npm run build
```

## 架构概览

### 前端（React + TypeScript）

**状态管理** - `src/stores/` (Zustand)
- `favoriteStore.ts`: 收藏列表
- `settingsStore.ts`: 应用设置（终端类型、通知等）

**Hooks** - `src/hooks/`
- `useRunningSessions.ts`: 运行中 session 状态管理
- `useSessionSearch.ts`: session 搜索逻辑
- `useNotification.ts`: Web Notifications API 封装

**服务层** - `src/services/` 封装 Tauri invoke 调用
- `claudeSession.ts`: session 数据操作
- `terminalService.ts`: 终端窗口跳转
- `notificationService.ts`: 通知服务

**组件** - `src/components/`
- `running/`: Running Tab（SessionCard, StatusBadge）
- `management/`: Session 管理 Tab（SessionList, SessionDetail, ConversationView）
- `dialogs/`: 对话框（NewSessionDialog, SettingsDialog, ConfirmDialog）
- `layout/`: 布局组件（AppLayout, TabHeader, SplitPane）
- `ui/`: shadcn/ui 基础组件

**数据请求** - `src/lib/query/` (TanStack Query)
- `queries.ts`: 查询定义
- `mutations.ts`: 变更定义
- `queryClient.ts`: QueryClient 配置

### 后端（Rust + Tauri）

**命令** - `src-tauri/src/commands/`
- `session.rs`: session 相关 Tauri 命令（init_running, list_running, get_conversation 等）
- `session_commands.rs`: 优化版 session 命令（list_sessions_optimized 等）
- `terminal.rs`: 终端窗口跳转命令（jump_to_terminal, resume_in_terminal）

**工具** - `src-tauri/src/utils/`
- `claude_data.rs`: 读取 Claude 数据目录，解析 JSONL 文件
- `claude_session.ts`: session 扫描优化实现
- `running_sessions.rs`: 运行中 session 状态管理
- `sessions_watcher.rs`: sessions 目录监听服务（notify crate）
- `window_manager.rs`: Windows 窗口管理（PID 匹配、进程链追踪）
- `logger.rs`: 日志系统配置
- `session_types.rs`: Session 类型定义
- `session_utils.rs`: Session 解析辅助函数

### 数据流

1. Claude Code 数据存储位置：
   - 项目 session: `~/.claude/projects/<project-name>/<session-id>.jsonl`
   - 运行中 session: `~/.claude/sessions/<pid>.json`

2. 状态监听机制：
   - Tauri 文件监听器监听 `~/.claude/sessions/` 目录变化
   - 检测文件创建/修改/删除事件
   - 通过 Tauri 事件系统通知前端

3. 前端通信：
   - `invoke()` 调用 Tauri 命令
   - `listen()` 接收状态变化事件（`running_sessions_changed`, `session_waiting_input`）

## 终端集成

终端跳转功能仅支持 Windows 平台：
- 支持终端类型：WezTerm、cmd、PowerShell
- 通过进程 PID 精确匹配终端窗口
- 通过进程链追踪找到 Claude 进程对应的终端
- 恢复 session 使用配置的终端启动命令

## 关键类型

**前端类型** - `src/types/`
- `session.ts`: ClaudeSession, SessionStatus, SessionMeta
- `conversation.ts`: Conversation, ConversationMessage
- `settings.ts`: AppSettings, TerminalType

**后端类型** - `src-tauri/src/utils/`
- `session_types.rs`: SessionMeta, SessionMessage
- `running_sessions.rs`: RunningSession, SessionStatus, SessionFileContent
- `claude_data.rs`: ClaudeSession, Conversation

## 注意事项

- 构建发布版本时需要同时完成前端 TypeScript 编译和 Rust 编译
- Windows 窗口跳转功能在非 Windows 平台会返回错误
- Session JSONL 文件解析逻辑在 `claude_data.rs` 的 `parse_session_file` 函数
- 运行中 session 检测通过检查进程 PID 是否存在（tasklist 命令）
- 日志文件存储在 `%APPDATA%/claude-fleet/logs/` 目录