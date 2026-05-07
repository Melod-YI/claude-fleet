# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Claude Fleet 是一个 Tauri 2.0 桌面应用，用于管理多个 Claude Code session。核心功能：
- 实时监控运行中的 session 状态，等待输入时发送通知
- 一键跳转到对应的 Windows Terminal 窗口（Windows 特有）
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
- **状态管理**: Zustand stores 在 `src/stores/`
  - `sessionStore.ts`: session 列表和当前对话
  - `favoriteStore.ts`: 收藏列表
  - `settingsStore.ts`: 应用设置
- **服务层**: `src/services/` 封装 Tauri invoke 调用
  - `claudeSession.ts`: session 数据操作
  - `terminalService.ts`: 终端窗口跳转
- **组件**: `src/components/`
  - `running/`: Running Tab 相关组件
  - `management/`: Session 管理 Tab 相关组件
  - `dialogs/`: 对话框组件
  - `ui/`: shadcn/ui 基础组件

### 后端（Rust + Tauri）
- **命令**: `src-tauri/src/commands/`
  - `session.rs`: session 相关 Tauri 命令
  - `terminal.rs`: 终端窗口跳转命令（Windows API）
- **工具**: `src-tauri/src/utils/`
  - `claude_data.rs`: 读取 Claude 数据目录（~/.claude/projects/, ~/.claude/sessions/）
  - `hooks.rs`: 文件监听钩子服务（notify crate）
  - `window_manager.rs`: Windows 窗口管理

### 数据流
1. Claude Code 数据存储在 `~/.claude/projects/<project-name>/<session-id>.jsonl` 和 `~/.claude/sessions/<session-id>.json`
2. 钩子机制：Claude Code hooks → Python script → `~/.claude-fleet/events/*.json` → Tauri file watcher → 前端事件
3. 前端通过 `invoke()` 调用 Tauri 命令，通过 `listen()` 接收事件

## 钩子配置

Claude Fleet 通过 Claude Code hooks 接收 session 状态变化。配置示例见 `docs/hooks/settings.example.json`。

用户需要在 `~/.claude/settings.json` 中配置 hooks，调用 `~/.claude-fleet/hook_writer.py`（应用启动时自动生成）。

钩子事件类型：`SessionStart`, `Stop`, `Notification`, `SessionEnd`

## Windows Terminal 集成

终端跳转功能仅支持 Windows 平台，使用 Windows API：
- 通过进程 PID 精确匹配终端窗口
- 通过工作目录路径模糊匹配
- 恢复 session 使用 `wt -d "路径" claude --resume <session-id>`

## 关键类型

Session 相关类型定义在 `src/types/session.ts`：
- `ClaudeSession`: session 元数据
- `SessionStatus`: 'running' | 'waiting_input' | 'completed' | 'idle'
- `SessionFilter`: 过滤条件

Tauri 后端对应类型在 `src-tauri/src/utils/claude_data.rs`。

## 注意事项

- 构建发布版本时需要同时完成前端 TypeScript 编译和 Rust 编译
- Windows 窗口跳转功能在非 Windows 平台会返回错误
- Session JSONL 文件解析逻辑在 `claude_data.rs` 的 `parse_session_file` 函数
- 运行中 session 检测通过检查进程 PID 是否存在（tasklist/ps 命令）