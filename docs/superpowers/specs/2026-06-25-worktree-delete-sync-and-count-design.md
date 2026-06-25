# Worktree 删除分支同步与计数徽标修复 设计

日期：2026-06-25
分支：worktree-enhance

## 背景

Worktree 功能存在两个待处理问题：

1. **删除 worktree 时不联动分支、无防丢失保护**：删除对话框 `deleteBranch` 硬编码为 `true`（`WorktreeTab.tsx:35`），后端 `delete_worktree` 用 `git worktree remove --force` + `git branch -D` 强制删除（`utils/git/worktree.rs:249,283`），对未提交变更和未合并提交无任何告警或阻断。用户希望删除「应用托管的 worktree」时同步删分支，且在分支有未提交变更或未合并进主干时告警。
2. **仓库 worktree 计数徽标显示错误**：`RepoTreeItem.tsx:32-34` 的 `useWorktreesQuery` 仅在 `expanded` 时启用，折叠时 `worktrees` 回退为 `[]`，导致徽标（`RepoTreeItem.tsx:57`）在折叠时始终显示 0，展开后才出现真实数字。

## 目标

- 删除托管 worktree 时同步删除其分支；删除前做安全预检，命中风险时告警并要求用户「强制删除」二次确认。
- 仓库折叠时也显示正确的 worktree 计数，性能开销可控。

## 非目标

- 不改变未托管 worktree 的分支归属策略（不替外部 worktree 删分支）。
- 不引入服务端强制阻断契约（阻断逻辑以前端预检 + 对话框为主，详见「设计决策」）。

## 设计决策

- **阻断语义**：未提交变更 / 未合并提交命中时，默认告警并阻止，但提供「强制删除」二次确认。不采用硬阻断（feature 分支常态即未合并，硬阻断会让删除按钮几乎不可用）。
- **二次确认形式**：单按钮明确文案「我已知晓风险，强制删除」即视为二次确认，不引入三层弹窗。
- **主干定义**：`get_default_branch`（main / master / develop 优先级回退）。
- **未托管 worktree**：删除时 `delete_branch=false`，仅移除目录；不执行未合并检查（分支非应用创建，归属不明）。但仍执行未提交变更检查（`git worktree remove --force` 会丢未提交工作，与是否托管无关）。
- **服务端契约**：`delete_worktree_cmd` 保持现状（已为强删），不新增 `force` 入参。阻断以预检命令 + 前端对话框为唯一关口。本地桌面应用，非安全边界。
- **计数方案**：新增轻量 `count_worktrees_cmd`，每仓库 1 次 git 子进程 + 1 次 DB 查询；不复用完整 `list_worktrees_cmd`（后者每 worktree 2 次子进程，开销过大）。

## 任务 1：删除同步分支 + 安全告警

### 数据流

```
点删除 → preflight_delete_worktree_cmd → DeletionSafety
  → 未阻断：普通确认框（删除目录{和分支}）
  → 阻断：风险告警框（原因列表 + 取消 + 强制删除）
→ delete_worktree_cmd(delete_branch = will_delete_branch)
```

### 后端

#### `utils/git/mod.rs` 新增

```rust
/// 判定 branch 是否已合并进 main_branch。
/// 返回 (is_merged, unmerged_commits)。
/// is_merged = branch 是 main 的祖先；unmerged_commits = main..branch 提交数。
pub fn is_branch_merged(
    repo_path: &Path,
    branch: &str,
    main_branch: &str,
) -> Result<(bool, u32), String>
```

实现：
- `git merge-base --is-ancestor <branch> <main_branch>`：退出码 0 → 已合并。
- `git rev-list --count main_branch..branch`：取未合并提交数。
- best-effort：失败时 `warn!` 并返回 `(false, 0)`，不阻断流程。

#### `commands/worktree.rs` 新增

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionSafety {
    pub is_managed: bool,
    pub will_delete_branch: bool,
    pub uncommitted_changes: u32,
    pub unmerged_commits: u32,
    pub blocked: bool,
    pub reasons: Vec<String>,
}

#[tauri::command]
pub fn preflight_delete_worktree_cmd(
    path: String,
    repo_path: String,
    branch: Option<String>,
) -> Result<DeletionSafety, String>
```

逻辑：
1. `is_managed` = DB `get_worktree_by_path(path)` 命中（有记录）。
2. `will_delete_branch` = `is_managed && branch.is_some() && branch_exists(repo, branch)`。
3. `uncommitted_changes` = `get_dirty_file_count(worktree_path)`（托管/未托管都查）。
4. `unmerged_commits` = 若 `will_delete_branch` 则 `is_branch_merged(repo, branch, default_branch)` 的第二项；否则 0。
5. `blocked` = `uncommitted > 0 || (will_delete_branch && unmerged > 0)`。
6. `reasons`：按命中项追加中文描述（如「N 个未提交变更」「N 个未合并到 <main> 的提交」）。
7. 日志：入口、各检查结果、blocked 与 reasons。

#### `delete_worktree_cmd` 不变

签名与实现保持现状。前端按 `will_delete_branch` 传入 `delete_branch`。

### 前端

#### `lib/api/worktrees.ts` 新增

```ts
preflightDeleteWorktree(path, repoPath, branch): Promise<DeletionSafety>
```

#### `WorktreeTab.tsx`

- `deleteConfirm` 状态扩展为：
  ```ts
  { open: boolean, worktree: WorktreeListItem | null, safety: DeletionSafety | null, force: boolean }
  ```
- `handleDeleteWorktree(worktree)`：先 `await preflightDeleteWorktree(...)`，设置 `safety`，打开对话框；失败 toast 并中止。
- `handleConfirmDelete`：用 `safety.will_delete_branch` 决定 `deleteBranch` 入参；`force` 仅影响 UI 文案与日志，不改变后端调用（后端本就强删）。

#### 对话框

`ConfirmDialog` 不足以展示原因列表 + 强制按钮。新增专用删除对话框组件 `components/worktree/DeleteWorktreeDialog.tsx`（或复用 `ErrorDialog` 的布局模式）：

- **未阻断态**：标题「删除 Worktree」；正文「将删除目录{和分支 `<branch>`}，此操作不可撤销。」；按钮「取消」「删除」。
- **阻断态**：标题「删除 Worktree - 存在风险」；正文红色原因列表（`safety.reasons`）；按钮「取消」「我已知晓风险，强制删除」（destructive variant）。

#### `worktreeMutations.ts`

`useDeleteWorktreeMutation` 无需改签名；`onSuccess` 额外 invalidate `["worktrees","count",repoPath]`（与任务 2 联动）。

### 测试

- `is_branch_merged`：单元测试覆盖 is-ancestor / 非 ancestor / 失败回退（用测试替身 git 输出）。
- `DeletionSafety` 序列化 camelCase + `blocked` 计算逻辑（托管/未托管 / 有无未提交 / 有无未合并组合）。
- 后端 `cargo test`。前端无测试基建，按 `npx tsc --noEmit` 通过为准。

## 任务 2：轻量计数 query

### 后端

#### `commands/worktree.rs` 新增

```rust
#[tauri::command]
pub fn count_worktrees_cmd(repo_path: String) -> Result<u32, String>
```

逻辑：
1. `list_worktrees_live(repo)` → 解析 porcelain，数 `!is_main` 条目（live 计数）。
2. DB `list_worktrees_by_repo(repo_path)` → 数「path 不在 live 集合」的记录（missing 计数）。
3. 返回 `live + missing`。
4. 日志：入口、live/missing/总数。

注册到 `lib.rs` invoke handler。

### 前端

#### `lib/api/worktrees.ts` 新增

```ts
countWorktrees(repoPath): Promise<number>
```

#### `lib/query/worktreeQueries.ts` 新增

```ts
export const useWorktreeCountQuery = (repoPath: string | undefined) =>
  useQuery<number>({
    queryKey: ["worktrees", "count", repoPath],
    queryFn: () => worktreesApi.countWorktrees(repoPath!),
    enabled: Boolean(repoPath),
    staleTime: 30 * 1000,
    refetchOnWindowFocus: false,
  })
```

#### `RepoTreeItem.tsx`

- 保留 `useWorktreesQuery(expanded ? repoPath : undefined)` 用于展开列表。
- 新增 `useWorktreeCountQuery(repoPath)`（始终启用）。
- 徽标改为读 count：`{count ?? 0}`；加载中可显示 `--` 或保持 0（采用 0 以避免闪烁）。

#### `worktreeMutations.ts`

- `useCreateWorktreeMutation.onSuccess`：额外 invalidate `["worktrees","count",repoPath]`。
- `useDeleteWorktreeMutation.onSuccess`：额外 invalidate `["worktrees","count",repoPath]`。
- `useRemoveTrackedRepoMutation`：无需改（已 remove `["worktrees",repoPath]`；count query 随仓库移除自然卸载）。

### 性能

N 个跟踪仓库 = N 次 git 子进程（`git worktree list --porcelain`），仅首次加载 + 30s 缓存 + 不随窗口聚焦 refetch。可接受。

### 测试

- `count_worktrees_cmd` 逻辑测试（mock porcelain 输出 + DB missing 计数）。
- 前端按 `npx tsc --noEmit` 通过为准。

## 变更文件清单

后端：
- `src-tauri/src/utils/git/mod.rs`（+ `is_branch_merged` + 测试）
- `src-tauri/src/commands/worktree.rs`（+ `DeletionSafety` + `preflight_delete_worktree_cmd` + `count_worktrees_cmd` + 测试）
- `src-tauri/src/lib.rs`（注册新命令）

前端：
- `src/lib/api/worktrees.ts`（+ preflight / count 调用 + `DeletionSafety` 类型）
- `src/types/worktree.ts`（+ `DeletionSafety` 类型导出）
- `src/lib/query/worktreeQueries.ts`（+ `useWorktreeCountQuery`）
- `src/lib/query/worktreeMutations.ts`（create/delete onSuccess 增 invalidate count）
- `src/components/worktree/RepoTreeItem.tsx`（徽标改读 count query）
- `src/components/worktree/DeleteWorktreeDialog.tsx`（新增）
- `src/components/worktree/WorktreeTab.tsx`（删除流程改预检 + 新对话框）

## 验证

1. `cd src-tauri && cargo test` 全绿。
2. `npx tsc --noEmit` 无错。
3. 手动验证（`npm run tauri dev`）：
   - 托管 worktree 有未提交变更 → 删除弹阻断框，强制删除后目录+分支均消失。
   - 托管 worktree 分支未合并 → 阻断框；合并后普通删除 → 分支同步删除。
   - 未托管 worktree 删除 → 仅删目录，分支保留。
   - 仓库折叠时徽标显示正确计数；新建/删除 worktree 后计数即时刷新。
