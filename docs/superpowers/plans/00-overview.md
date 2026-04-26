# Claude Fleet 实现计划总览

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement plans task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## 项目目标

构建一个桌面应用，管理多个 Claude Code session：
- 监控运行中 session 状态，等待输入时主动提示
- 快速跳转到对应的 Windows Terminal 窗口
- 管理、搜索、恢复历史 session

## 技术栈

- **前端**: React + TypeScript + Tailwind CSS + shadcn/ui
- **桌面框架**: Tauri 2.0
- **状态管理**: Zustand
- **构建工具**: Vite

## 文件结构

```
claude-fleet-sp/
├── src/                          # 前端源码
│   ├── components/               # UI 组件
│   │   ├── layout/               # 布局组件
│   │   │   ├── AppLayout.tsx     # 主布局（Tab 切换）
│   │   │   ├── SplitPane.tsx     # 左右分栏容器
│   │   │   └── TabHeader.tsx     # Tab 导航头
│   │   ├── running/              # "运行中" Tab 组件
│   │   │   ├── RunningTab.tsx    # 运行中 Tab 主组件
│   │   │   ├── SessionCard.tsx   # Session 卡片
│   │   │   └── StatusBadge.tsx   # 状态徽章
│   │   ├── management/           # "Session 管理" Tab 组件
│   │   │   ├── ManagementTab.tsx # Session 管理 Tab 主组件
│   │   │   ├── SessionList.tsx   # 左侧列表
│   │   │   ├── SessionListItem.tsx # 列表项
│   │   │   ├── SessionDetail.tsx # 右侧详情
│   │   │   ├── ConversationView.tsx # 对话历史视图
│   │   │   ├── DirectoryTree.tsx # 目录树视图
│   │   │   └── SearchBar.tsx     # 搜索栏
│   │   ├── dialogs/              # 弹窗组件
│   │   │   ├── NewSessionDialog.tsx # 新建 session 弹窗
│   │   │   └── ConfirmDialog.tsx # 确认弹窗
│   │   └── common/               # 公共组件
│   │   │   ├── Button.tsx        # 按钮
│   │   │   ├── Input.tsx         # 输入框
│   │   │   ├── Toggle.tsx        # 开关
│   │   │   ├── Badge.tsx         # 徽章
│   │   │   └── ScrollArea.tsx    # 滚动区域
│   ├── hooks/                    # React hooks
│   │   ├── useSessions.ts        # session 数据 hook
│   │   ├── useFavorites.ts       # 收藏管理 hook
│   │   ├── useSearch.ts          # 搜索 hook
│   │   └── useNotification.ts    # 通知 hook
│   ├── stores/                   # Zustand stores
│   │   ├── sessionStore.ts       # session 状态
│   │   ├── favoriteStore.ts      # 收藏状态
│   │   └── settingsStore.ts      # 设置状态
│   ├── services/                 # 服务层
│   │   ├── claudeSession.ts      # Claude session 数据服务
│   │   ├── terminalService.ts    # 终端窗口管理服务
│   │   ├── notificationService.ts # 通知服务
│   │   └── hookReceiver.ts       # 钩子接收服务
│   ├── types/                    # TypeScript 类型定义
│   │   ├── session.ts            # Session 类型
│   │   ├── conversation.ts       # 对话类型
│   │   └── settings.ts           # 设置类型
│   ├── utils/                    # 工具函数
│   │   ├── pathUtils.ts          # 路径处理
│   │   ├── timeUtils.ts          # 时间处理
│   │   └── fuzzySearch.ts        # 模糊搜索
│   ├── App.tsx                   # 应用入口
│   ├── main.tsx                  # React 入口
│   └── index.css                 # 全局样式
├── src-tauri/                    # Tauri 后端
│   ├── src/                      # Rust 源码
│   │   ├── main.rs               # Tauri 入口
│   │   ├── commands/             # Tauri commands
│   │   │   ├── session.rs        # session 相关命令
│   │   │   ├── terminal.rs       # 终端窗口命令
│   │   │   ├── notification.rs   # 通知命令
│   │   │   └ hooks.rs            # 钩子处理命令
│   │   ├── lib.rs                # 库入口
│   │   └── utils/                # Rust 工具
│   │       ├── claude_data.rs    # Claude 数据读取
│   │       ├── window_manager.rs # 窗口管理
│   │       └ hooks.rs            # 钩子处理
│   ├── tauri.conf.json           # Tauri 配置
│   ├── Cargo.toml                # Rust 依赖
│   └── icons/                    # 应用图标
├── tests/                        # 测试
│   ├── unit/                     # 单元测试
│   └── e2e/                      # E2E 测试
├── docs/                         # 文档
│   └── superpowers/
│       ├── specs/                # 设计文档
│       └── plans/                # 实现计划
├── package.json                  # npm 配置
├── vite.config.ts                # Vite 配置
├── tailwind.config.js            # Tailwind 配置
└── tsconfig.json                 # TypeScript 配置
```

## 实现阶段

| Phase | 文件 | 内容 | 预计工作量 |
|-------|------|------|-----------|
| 1 | `phase-01-project-init.md` | Tauri + React 项目初始化 | 2-3h |
| 2 | `phase-02-data-layer.md` | Session 数据读取和管理 | 3-4h |
| 3 | `phase-03-running-tab.md` | "运行中" Tab UI | 2-3h |
| 4 | `phase-04-management-list.md` | "Session 管理" 左侧列表 | 3-4h |
| 5 | `phase-05-management-detail.md` | "Session 管理" 右侧详情 | 2-3h |
| 6 | `phase-06-new-session.md` | 新建 Session 弹窗 | 2h |
| 7 | `phase-07-hooks-notification.md` | 钩子集成和通知 | 4-5h |
| 8 | `phase-08-terminal-jump.md` | 跳转终端功能 | 2-3h |
| 9 | `phase-09-integration.md` | 最终集成和测试 | 2-3h |

## 执行顺序

每个 Phase 产出的都是可测试、可运行的代码。按顺序执行：
1. Phase 1-2：基础设施，必须先完成
2. Phase 3-6：UI 功能，可以并行开发（但有依赖顺序）
3. Phase 7-8：核心功能（钩子、跳转），依赖数据层
4. Phase 9：最终集成

## 详细计划文件

各 Phase 的详细任务见对应的计划文件：
- [Phase 1: 项目初始化](phase-01-project-init.md)
- [Phase 2: Session 数据层](phase-02-data-layer.md)
- [Phase 3: "运行中" Tab](phase-03-running-tab.md)
- [Phase 4: "Session 管理" Tab - 列表](phase-04-management-list.md)
- [Phase 5: "Session 管理" Tab - 详情](phase-05-management-detail.md)
- [Phase 6: 新建 Session](phase-06-new-session.md)
- [Phase 7: 钩子和通知](phase-07-hooks-notification.md)
- [Phase 8: 跳转终端](phase-08-terminal-jump.md)
- [Phase 9: 最终集成](phase-09-integration.md)