# Worktree 管理页面 UI 设计

## 概述

为 Claude Fleet 新增 Worktree 管理页面，作为第二个 Tab（运行中 → **Worktree** → Session 管理）。用户可以管理多个 git 仓库的 worktree，创建新 worktree，并在 worktree 中运行 Claude Code。

## 页面结构

### Tab 导航

在现有 `AppLayout.tsx` 的 `TABS` 数组中插入新 tab：

```ts
const TABS = [
  { id: "running", label: "运行中" },
  { id: "worktree", label: "Worktree" },      // 新增
  { id: "management", label: "Session 管理" },
]
```

`App.tsx` 增加条件渲染：`{activeTab === "worktree" && <WorktreeTab />}`

### 整体布局

左右分栏（复用 `SplitPane` 组件模式）：
- **左侧栏**（~220px）：仓库目录树，可展开/收起
- **右侧面板**：工具栏 + worktree 详情 + 操作按钮

## 左侧栏：仓库目录树

### 结构

```
仓库列表                          [＋ 添加]
─────────────────────────────
▼ 📁 claude-fleet-sp           [3] [✕]
    🌿 feature-auth              ← 选中高亮
    🌿 fix-memory-leak
    🌿 old-experiment            ← Missing 状态半透明
    [＋ 新建 worktree]           ← 虚线按钮
▶ 📁 another-project           [2] [✕]
▶ 📁 my-lib                    [1] [✕]
─────────────────────────────
[＋ 添加仓库]                    ← 底部虚线框
```

### 仓库管理

**添加仓库**：
- 入口：顶部「＋」按钮 或 底部虚线框
- 交互：调用 `@tauri-apps/plugin-dialog` 的 `open({ directory: true })` 选择目录
- 验证：后端检查是否为 git 仓库（`git rev-parse --is-inside-work-tree`）
- 去重：DB 的 `path` 字段有 UNIQUE 约束，重复添加时前端 toast 提示「该仓库已在列表中」
- 名称：自动从目录名提取（复用后端 `get_repo_name`）
- 存储：新增 `tracked_repos` 表持久化

**删除仓库**：
- 入口：仓库行右侧「✕」按钮（hover 时显示，默认低透明度）
- 交互：弹出确认对话框（`ConfirmDialog`）
- 行为：仅从跟踪列表移除，**不删除本地文件**，不影响 worktree
- 提示文案：「将从列表中移除此仓库，不会删除本地文件。」

**展开/收起**：
- 点击仓库行切换展开状态
- 展开后显示该仓库下所有 worktree
- 数字 badge 显示 worktree 数量（不含 main worktree）

### 仓库列表来源

新增 `tracked_repos` 数据库表：

```sql
CREATE TABLE tracked_repos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    added_at INTEGER NOT NULL
);
```

对应 Tauri 命令：
- `add_tracked_repo(path) → TrackedRepo`
- `remove_tracked_repo(id)`
- `list_tracked_repos() → Vec<TrackedRepo>`

`TrackedRepo` 类型：

```rust
struct TrackedRepo {
    id: i64,
    path: String,
    name: String,
    added_at: i64,
}
```

前端对应 TypeScript 类型：

```ts
interface TrackedRepo {
  id: number
  path: string
  name: string
  addedAt: number
}
```

## Worktree 列表展示

### 数据来源

复用现有 `list_worktrees_cmd(repo_path)` 返回 `Vec<WorktreeListItem>`。

每个仓库展开时调用一次，TanStack Query 缓存。

### 显示规则

- **Active**（DB + git 都存在）：正常显示，绿色状态
- **Unmanaged**（仅 git 存在，未在 DB 注册）：正常显示，黄色提示
- **Missing**（仅 DB 存在，git 已删除）：半透明显示，红色状态
- **is_main** 的 worktree 不显示（后端已过滤）
- 排序：Active → Unmanaged → Missing，组内按创建时间倒序

### 新建 worktree 入口

展开仓库后，worktree 列表底部显示「＋ 新建 worktree」虚线按钮。点击弹出创建对话框，自动关联当前仓库。

## 右侧面板

### 工具栏

选中仓库后显示：
- **「＋ 新建 Worktree」**：主色按钮，弹出创建对话框（自动关联当前选中仓库）
- **「🔄 刷新」**：outline 按钮，重新获取当前仓库的 worktree 列表和 git 状态

未选中仓库时工具栏不显示。

### 空状态

未选中 worktree 时显示居中提示：「选择一个 worktree 查看详情，或点击新建」

### 详情区域

选中 worktree 后展示：

**标题区**：
- worktree 名称（大字号 + 粗体）
- 状态 badge：`Active`（绿）/ `Missing`（红）/ `Unmanaged`（黄）

**基本信息卡片**（背景色区块）：
| 字段 | 说明 |
|------|------|
| 路径 | worktree 目录完整路径，monospace 字体 |
| 分支 | 当前分支名 |
| 基于 | 创建时的基准 ref（如 origin/main） |
| 创建时间 | 格式化日期时间 |

**Git 状态卡片**（需要新增后端 API）：
| 指标 | 说明 | 颜色 |
|------|------|------|
| ahead | 领先远程的 commit 数 | 绿色 |
| behind | 落后远程的 commit 数 | 红色 |
| 未提交变更 | 工作区 + 暂存区修改文件数 | 黄色 |

> **注意**：Git 状态需要新增后端 API。第一期可以先不实现 git status 获取，显示为「--」占位，后续补充。

**操作按钮行**：
| 按钮 | 行为 | 样式 |
|------|------|------|
| ▶ 运行 Claude Code | 调用 `launch_session`，worktree path 作为 working_directory | 主色（violet） |
| 📂 打开目录 | 调用 `open_directory` | outline |
| 💻 VS Code | 调用 `open_in_vscode` | outline |
| 🗑 删除 | 弹出确认对话框 → 调用后端删除 | 红色（destructive） |

Missing 状态的 worktree：禁用「运行 Claude Code」和「打开目录」「VS Code」按钮，仅保留「删除」。

> **第一期实现说明**：删除按钮在 UI 中展示，但处于禁用状态（`disabled`），tooltip 提示「功能开发中」。第二期后端 API 就绪后启用。

## 创建 Worktree 对话框

### 交互：智能表单

复用现有 `Dialog` 组件模式（参考 `NewSessionDialog`）。

**默认视图**：
- 标题：「新建 Worktree」
- 一个输入框：Worktree 名称（必填，自动 focus）
- 信息提示条（紫色底色）：显示自动生成的配置 `分支：<name> · 基于：origin/main`
- 「▶ 高级选项」折叠按钮
- 底部：取消 + 创建按钮

**展开高级选项后**：
- 分支名输入框（留空则同名称）
- 基于分支下拉选择器（调用 `get_repo_info_cmd` 获取可选分支列表）
  - 分组显示：远程分支（origin/xxx）、本地分支、upstream 分支
  - 默认选中：`origin/main` 或 `origin/<default_branch>`

**名称验证**：
- 不能为空
- 不能包含 Windows 路径非法字符（`\ / : * ? " < > |`）
- 不能和已有 worktree 重名

**创建流程**：
1. 调用 `create_worktree_cmd(repo_path, name, branch, base_ref)`
2. 成功后刷新 worktree 列表（invalidate TanStack Query）
3. 关闭对话框
4. 自动选中新创建的 worktree

### 前端类型（已有）

```ts
// src/types/worktree.ts - 已有
interface RepoInfo {
  name: string
  remotes: RemoteInfo[]
  localBranches: string[]
  remoteBranches: string[]
  defaultBranch: string
}
```

## 删除 Worktree

### 交互

1. 点击「🗑 删除」按钮
2. 弹出 `ConfirmDialog`：
   - 标题：「删除 Worktree」
   - 描述：「将删除 worktree 目录 `<path>` 和分支 `<branch>`。此操作不可恢复。」
   - 确认按钮：红色 destructive 样式
3. 调用后端 `delete_worktree_cmd`（第二期实现）
4. 成功后刷新列表

### 后端 API（第二期）

```rust
#[tauri::command]
fn delete_worktree_cmd(repo_path: String, name: String) -> Result<(), String>
```

逻辑：
1. `git worktree remove <path> --force`
2. `git branch -d <branch>`（如果有未合并变更，改用 `-D` 并警告）
3. 删除 DB 记录

## 前端组件结构

```
src/components/worktree/
├── WorktreeTab.tsx          # 主页面组件，管理整体状态
├── RepoTree.tsx             # 左侧仓库目录树
├── RepoTreeItem.tsx         # 单个仓库行（展开/收起 + badge + 删除）
├── WorktreeTreeItem.tsx     # 单个 worktree 行（选中高亮）
├── WorktreeDetail.tsx       # 右侧详情面板
├── WorktreeToolbar.tsx      # 右侧工具栏
├── CreateWorktreeDialog.tsx # 创建对话框（智能表单）
└── EmptyState.tsx           # 空状态提示
```

### 数据流

```
WorktreeTab
├── useTrackedReposQuery()     → list_tracked_repos
├── useWorktreesQuery(repoPath) → list_worktrees (per repo, enabled when expanded)
├── useRepoInfoQuery(repoPath)  → get_repo_info (for create dialog)
├── useGitStatusQuery(wtPath)   → get_git_status (for detail panel, phase 2)
│
├── RepoTree
│   ├── RepoTreeItem (per repo)
│   │   └── WorktreeTreeItem (per worktree)
│   └── AddRepoButton
│
├── WorktreeDetail (when worktree selected)
│   ├── WorktreeToolbar
│   └── WorktreeInfo / GitStatus / ActionButtons
│
└── CreateWorktreeDialog (modal)
```

### 状态管理

- **仓库列表**：TanStack Query（`useQuery` + `useMutation`）
- **Worktree 列表**：TanStack Query，per-repo query key `[worktrees, repoPath]`
- **展开状态**：组件内 `useState<Set<string>>`（展开的 repo path 集合）
- **选中状态**：组件内 `useState<WorktreeListItem | null>`
- **对话框**：组件内 `useState<boolean>`

### 服务层

新增 `src/lib/api/worktrees.ts`：

```ts
export const worktreeApi = {
  listTrackedRepos: () => invoke<TrackedRepo[]>('list_tracked_repos'),
  addTrackedRepo: (path: string) => invoke<TrackedRepo>('add_tracked_repo', { path }),
  removeTrackedRepo: (id: number) => invoke('remove_tracked_repo', { id }),
  listWorktrees: (repoPath: string) => invoke<WorktreeListItem[]>('list_worktrees_cmd', { repoPath }),
  getRepoInfo: (repoPath: string) => invoke<RepoInfo>('get_repo_info_cmd', { repoPath }),
  createWorktree: (repoPath: string, name: string, branch: string, baseRef: string) =>
    invoke<WorktreeInfo>('create_worktree_cmd', { repoPath, name, branch, baseRef }),
}
```

## 后端新增

### 第一期（本次实现）

1. **tracked_repos 表** + CRUD Tauri 命令
2. **DB schema 迁移**：在 `schema.rs` 增加 `tracked_repos` 建表语句

### 第二期（后续）

1. **delete_worktree_cmd**：删除 worktree + 分支 + DB 记录
2. **get_git_status_cmd**：获取 ahead/behind/未提交变更数
3. **merge_worktree_cmd**：合并 worktree 到基础分支

## 第一期实现范围

### 包含

- [x] WorktreeTab 页面（第二个 Tab，位于运行中和 Session 管理之间）
- [x] 左侧仓库目录树（添加/删除仓库、展开/收起）
- [x] Worktree 列表展示（Active/Missing/Unmanaged 状态）
- [x] 右侧详情面板（基本信息 + 操作按钮）
- [x] 创建 Worktree 智能表单对话框
- [x] 运行 Claude Code（直接 launch_session）
- [x] 打开目录 / VS Code
- [x] tracked_repos 数据库表 + CRUD
- [x] TanStack Query 数据获取
- [x] 删除 Worktree 按钮（UI 就绪，但按钮禁用，待第二期后端 API 实现后启用）
- [x] Git 状态区域（UI 就绪，显示「--」占位，待第二期后端 API 实现后填充数据）

### 不包含（后续迭代）

- [ ] Git 状态实际数据获取（ahead/behind/未提交）— 第二期后端
- [ ] 删除 Worktree 后端实现（git worktree remove + branch delete + DB 清理）— 第二期
- [ ] 合并 Worktree — 第二期后端实现
- [ ] Worktree sync — 第四期
- [ ] Claude 运行状态检测（是否正在运行 Claude Code）— 后续关联 running sessions
