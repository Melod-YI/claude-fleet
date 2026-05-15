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

## 音频资源嵌入（便携版支持）

音频文件通过 `include_bytes!` 在编译时嵌入到 exe 二进制中，实现免安装便携版。

**实现要点：**
- `build.rs` 扫描 `src-tauri/sounds/` 目录，生成 `embedded_sounds.rs`
- `include_bytes!` 的路径是相对于生成的代码文件位置（OUT_DIR），而非项目根目录
- OUT_DIR 位于 `target/release/build/<hash>/out/`，需使用 `../../../sounds/` 回退到正确路径
- 使用 `#[cfg(not(debug_assertions))]` 区分开发模式（读文件）和生产模式（读嵌入数据）
- 同时支持外部 `sounds/` 目录用于用户自定义音频扩展

**相关文件：**
- `src-tauri/build.rs`: 生成嵌入代码
- `src-tauri/src/commands/sound.rs`: 音频读取逻辑

## 开发规范

### 日志规范

后端代码需要提供足够丰富的日志，便于问题定位：

**必须添加日志的位置：**
1. **重要方法入口** - 记录方法开始执行和关键参数
2. **重要方法结束** - 记录执行结果和耗时（如适用）
3. **核心业务分支** - 条件分支的决策点和结果
4. **错误处理** - 捕获异常时记录完整错误信息
5. **状态变化** - session 状态变更、配置修改等

**日志级别使用：**
- `info!`: 正常业务流程、重要操作结果
- `debug!`: 详细执行过程、中间状态（需开启 DEBUG）
- `warn!`: 非预期但可恢复的情况、功能不支持
- `error!`: 错误、异常、操作失败

**日志格式示例：**
```rust
// 方法入口
info!("[method_name] 开始，参数: {}", param);

// 条件分支
debug!("[method_name] 分支A: 条件满足");
info!("[method_name] 执行成功，结果: {}", result);

// 错误
error!("[method_name] 失败: {}", e);
```

**日志文件位置：**
- `%USERPROFILE%\.claude-fleet\logs\claude-fleet-YYYY-MM-DD.log`
- 保留最近 7 天日志

### 前端规范

**技术栈：**
- React + TypeScript + Tailwind CSS + shadcn/ui（默认）
- 状态管理：Zustand
- 数据请求：TanStack Query

**组件命名：**
- 组件文件使用 PascalCase：`SessionCard.tsx`
- 组件函数使用 PascalCase：`function SessionCard()`

**样式规范：**
- 使用 Tailwind CSS 类名
- 使用 `cn()` 函数组合类名：`cn("base-class", condition && "conditional-class")`
- 遵循 shadcn/ui 组件风格

**服务层规范：**
- 封装 Tauri `invoke()` 调用
- 统一错误处理：`throw new Error(`操作失败: ${error}`)`
- 使用 async/await

### 后端规范

**技术栈：**
- Rust + Tauri 2.0
- 日志：tracing + tracing_subscriber

**命令命名：**
- Tauri 命令使用 snake_case：`open_directory`
- 前端调用对应 camelCase：`openDirectory`

**Windows 特定代码：**
- 使用 `#[cfg(target_os = "windows")]` 条件编译
- 非 Windows 平台返回 `"仅支持 Windows 平台"` 错误
- 使用 `cmd.exe` 执行外部命令，确保继承完整 PATH 环境变量

**Windows 外部命令执行经验总结：**

Windows GUI 应用（如 Tauri）可能不继承终端的完整 PATH 环境变量，导致直接调用 PATH 中的命令失败。以下是解决方案的演进过程：

1. **直接调用（失败）** - 找不到 PATH 中的命令
   ```rust
   Command::new("code").arg(&path).spawn()  // ❌ 失败
   ```

2. **通过 cmd.exe + start（能执行但有窗口）** - 弹出终端窗口且不会自动关闭
   ```rust
   Command::new("cmd.exe")
       .args(["/C", "start", "code", &path])
       .spawn()  // ❌ 有窗口闪烁
   ```

3. **通过 cmd.exe 直接执行（窗口短暂显示）** - 终端会退出但窗口短暂可见
   ```rust
   Command::new("cmd.exe")
       .args(["/C", "code", &path])
       .spawn()  // ❌ 窗口短暂显示
   ```

4. **最终方案：CREATE_NO_WINDOW 标志** - 完全隐藏进程窗口
   ```rust
   use std::os::windows::process::CommandExt;
   const CREATE_NO_WINDOW: u32 = 0x08000000;

   Command::new("cmd.exe")
       .args(["/C", "code", &path])
       .creation_flags(CREATE_NO_WINDOW)  // ✅ 完全隐藏窗口
       .spawn()
   ```

**关键要点：**
- `CREATE_NO_WINDOW = 0x08000000` 是 Windows API 标志，完全隐藏进程窗口
- `cmd.exe /C` 执行命令后自动退出，配合 CREATE_NO_WINDOW 无任何窗口显示
- `start` 命令会在新窗口启动程序，即使程序退出窗口也会短暂显示，**不要使用**
- 需要引入 `std::os::windows::process::CommandExt` trait 才能使用 `creation_flags`

### 测试规范

**端到端测试：**
- 使用 webapp-testing skill 完成端到端测试
- 设计日志时考虑通过日志定位问题

**单元测试：**
- 运行：`cd src-tauri && cargo test`
- 前端：`npm run build` 检查 TypeScript 编译

## 注意事项

- 构建发布版本时需要同时完成前端 TypeScript 编译和 Rust 编译
- Windows 窗口跳转功能在非 Windows 平台会返回错误
- Session JSONL 文件解析逻辑在 `claude_data.rs` 的 `parse_session_file` 函数
- 运行中 session 检测通过检查进程 PID 是否存在（tasklist 命令）
- 日志文件存储在 `%USERPROFILE%\.claude-fleet\logs\` 目录
- Windows GUI 应用可能不继承完整 PATH，使用 `cmd.exe` 执行外部命令