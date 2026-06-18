# Git Worktree 功能路线图

## 总目标

为 Claude Fleet 添加完整的 git worktree 管理能力，让用户可以在 UI 中：

- 查看指定 git 项目下的所有 worktree 信息
- 创建、删除、合并 worktree
- 在指定的 worktree 下启动 Claude Code

### 业务场景

1. **个人项目** — 基于本地默认分支创建 worktree，最终 merge 回默认分支并推送到 origin
2. **开源项目（fork 模式）** — upstream 为原始仓库，origin 为自己的 fork，需要基于 upstream 的分支创建 worktree

### 分支策略

底层 API 支持基于任意 `base_ref`（如 `origin/main`、`upstream/main`）创建 worktree。上层逻辑按以下优先级自动选择：

```
upstream → origin → 本地默认分支
```

用户可手动覆盖选择。前端在用户做出非默认选择时给出提示。

---

## 已完成

### 第一期：后端创建 + 列表（2026-06-18）

| 功能 | 状态 | 说明 |
|---|---|---|
| worktrees 数据库表 | ✅ | SQLite，含 name/branch/path/repo_name/repo_path/base_ref/created_at |
| DB CRUD | ✅ | insert / list_by_repo / get_by_path / delete_by_path |
| Git 工具层 | ✅ | execute_git（`git -C` 模式）、get_repo_name、get_remotes、get_local/remote_branches、get_default_branch、branch_exists、get_repo_parent |
| Worktree 创建逻辑 | ✅ | 验证仓库 → 获取名称 → 计算绝对路径 → 冲突检查 → 创建分支 → worktree add → 复制 .claude 目录 |
| Worktree 列表（实时） | ✅ | 解析 `git worktree list --porcelain` |
| .claude 目录复制 | ✅ | 递归复制主仓库的 .claude 到 worktree |
| Tauri 命令: create_worktree_cmd | ✅ | 创建 + 持久化 + 输入验证 |
| Tauri 命令: list_worktrees_cmd | ✅ | 融合 DB 记录 + git 实时状态，标记 Active/Missing/Unmanaged |
| Tauri 命令: get_repo_info_cmd | ✅ | 返回 remotes、branches、defaultBranch |
| 前端 TypeScript 类型 | ✅ | RemoteInfo, RepoInfo, WorktreeInfo, WorktreeListItem, WorktreeStatus |
| 单元测试 | ✅ | 33 个测试全部通过 |

**关键设计决策：**
- Worktree 目录位置：`../<repo>.worktrees/<name>`（主仓库父目录下）
- 使用绝对路径调用 `git worktree add`
- 数据库记录 `base_ref` 字段，为未来 sync 功能预留
- WorktreeStatus 枚举使用 `lowercase` serde 序列化

**Commits:** `ad94f4b` → `00afec0`（8 commits）

---

## 下一步

### 第二期：后端删除 + 合并

- [ ] 删除 worktree（安全检查：未提交更改、未推送提交、分支合并状态）
- [ ] 合并 worktree 到基础分支（支持 ff-only / merge / squash 策略）
- [ ] 合并后自动清理（可选）
- [ ] 删除对应的数据库记录

### 第三期：前端 UI

- [ ] Worktree 列表页面（展示 Active/Missing/Unmanaged 状态）
- [ ] 创建 worktree 对话框（仓库选择 → 分支选择器 → 名称输入）
- [ ] 分支选择器组件（基于 get_repo_info 数据，展示 upstream/origin/local 分组）
- [ ] 非默认选择警告提示
- [ ] 删除/合并操作 UI
- [ ] 在 worktree 目录启动 Claude Code（复用 launch_session）

### 第四期：高级功能

- [ ] Worktree sync（基于记录的 base_ref 同步更新）
- [ ] 批量操作
- [ ] Worktree 与 session 关联（展示哪些 worktree 正在运行 Claude Code）
- [ ] Submodule 支持
- [ ] Worktree 重命名

---

## 文件索引

| 文件 | 职责 |
|---|---|
| `src-tauri/src/utils/git/mod.rs` | 通用 git 命令封装 |
| `src-tauri/src/utils/git/worktree.rs` | Worktree 业务逻辑 |
| `src-tauri/src/db/worktrees.rs` | Worktrees 表 CRUD |
| `src-tauri/src/commands/worktree.rs` | Tauri invoke 命令 |
| `src/types/worktree.ts` | 前端类型定义 |
| `docs/superpowers/specs/2026-06-18-worktree-backend-design.md` | 第一期设计文档 |
| `docs/superpowers/plans/2026-06-18-worktree-backend-phase1.md` | 第一期实现计划 |
