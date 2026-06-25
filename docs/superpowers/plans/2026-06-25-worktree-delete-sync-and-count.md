# Worktree 删除分支同步与计数徽标修复 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 删除托管 worktree 时同步删分支并以预检告警+强制二次确认防丢失；修复仓库折叠时 worktree 计数徽标恒为 0 的问题。

**Architecture:** 后端新增 `preflight_delete_worktree_cmd`（返回 `DeletionSafety`）与 `count_worktrees_cmd`（轻量计数），纯逻辑提取为可测函数；前端新增 `useWorktreeCountQuery` 始终启用，删除流程改为预检→三态对话框→执行。

**Tech Stack:** Rust + Tauri 2.0（后端，`tracing` 日志，`cargo test`），React + TypeScript + TanStack Query（前端，`npx tsc --noEmit` 校验）。

参考 spec：`docs/superpowers/specs/2026-06-25-worktree-delete-sync-and-count-design.md`

---

## File Structure

后端：
- `src-tauri/src/utils/git/mod.rs` — 新增 `parse_rev_list_count`（纯函数）+ `is_branch_merged`
- `src-tauri/src/commands/worktree.rs` — 新增 `DeletionSafety`、`compute_deletion_safety_fields`（纯函数）、`preflight_delete_worktree_cmd`、`count_live_worktrees`（纯函数）、`count_worktrees_cmd`
- `src-tauri/src/lib.rs` — 注册两个新命令

前端：
- `src/types/worktree.ts` — 新增 `DeletionSafety` 类型
- `src/lib/api/worktrees.ts` — 新增 `preflightDeleteWorktree`、`countWorktrees`
- `src/lib/query/worktreeQueries.ts` — 新增 `useWorktreeCountQuery`
- `src/lib/query/worktreeMutations.ts` — create/delete `onSuccess` 增 invalidate count
- `src/components/worktree/RepoTreeItem.tsx` — 徽标改读 count query
- `src/components/worktree/DeleteWorktreeDialog.tsx` — 新增三态删除对话框
- `src/components/worktree/WorktreeTab.tsx` — 删除流程改预检 + 新对话框
- `src/components/worktree/index.ts` — 导出 `DeleteWorktreeDialog`

---

## Task 1: `parse_rev_list_count` + `is_branch_merged`（后端 git 工具）

**Files:**
- Modify: `src-tauri/src/utils/git/mod.rs`（在 `get_dirty_file_count` 之后、`#[cfg(test)]` 之前新增；测试加在 `mod tests` 内）

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/utils/git/mod.rs` 的 `mod tests` 末尾追加：

```rust
    #[test]
    fn parse_rev_list_count_parses_number() {
        assert_eq!(parse_rev_list_count("3"), 3);
    }

    #[test]
    fn parse_rev_list_count_trims_whitespace() {
        assert_eq!(parse_rev_list_count("  12 \n"), 12);
    }

    #[test]
    fn parse_rev_list_count_zero_when_empty() {
        assert_eq!(parse_rev_list_count(""), 0);
    }

    #[test]
    fn parse_rev_list_count_zero_when_non_numeric() {
        assert_eq!(parse_rev_list_count("abc"), 0);
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test parse_rev_list_count -- --nocapture`
Expected: 编译失败，`parse_rev_list_count` 未定义。

- [ ] **Step 3: 写实现**

在 `get_dirty_file_count` 函数之后插入：

```rust
/// 解析 `git rev-list --count <range>` 输出为 u32。
/// 空或非数字返回 0。
pub fn parse_rev_list_count(output: &str) -> u32 {
    output.trim().parse::<u32>().unwrap_or(0)
}

/// 判定 branch 相对 main_branch 的合并状态。
/// 返回 (is_merged, unmerged_commits)。
/// unmerged_commits = `git rev-list --count main..branch`（branch 有而 main 没有的提交数）。
/// best-effort：git 失败时返回 (false, 0)，不阻断删除流程。
pub fn is_branch_merged(
    repo_path: &Path,
    branch: &str,
    main_branch: &str,
) -> Result<(bool, u32), String> {
    let range = format!("{}..{}", main_branch, branch);
    match execute_git(repo_path, &["rev-list", "--count", &range]) {
        Ok(output) => {
            let n = parse_rev_list_count(&output);
            Ok((n == 0, n))
        }
        Err(e) => {
            warn!("[is_branch_merged] rev-list 失败，按不阻断处理: {}", e);
            Ok((false, 0))
        }
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test parse_rev_list_count -- --nocapture`
Expected: 4 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/utils/git/mod.rs
git commit -m "feat(git): add is_branch_merged + parse_rev_list_count helper"
```

---

## Task 2: `DeletionSafety` 结构 + `compute_deletion_safety_fields` 纯函数

**Files:**
- Modify: `src-tauri/src/commands/worktree.rs`（在 `RepoInfo` 结构之后新增结构体与函数；测试加在 `mod tests` 内）

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/commands/worktree.rs` 的 `mod tests` 末尾追加：

```rust
    #[test]
    fn deletion_safety_camel_case_roundtrip() {
        let s = DeletionSafety {
            is_managed: true,
            will_delete_branch: true,
            uncommitted_changes: 2,
            unmerged_commits: 3,
            blocked: true,
            reasons: vec!["未提交".to_string()],
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("isManaged"));
        assert!(json.contains("willDeleteBranch"));
        assert!(json.contains("uncommittedChanges"));
        assert!(json.contains("unmergedCommits"));
        assert!(!json.contains("uncommitted_changes"));
    }

    #[test]
    fn compute_safety_not_blocked_when_clean() {
        let (blocked, reasons) = compute_deletion_safety_fields(0, 0, true);
        assert!(!blocked);
        assert!(reasons.is_empty());
    }

    #[test]
    fn compute_safety_blocked_by_uncommitted() {
        let (blocked, reasons) = compute_deletion_safety_fields(5, 0, true);
        assert!(blocked);
        assert!(reasons.iter().any(|r| r.contains("未提交") && r.contains("5")));
    }

    #[test]
    fn compute_safety_blocked_by_unmerged_when_will_delete_branch() {
        let (blocked, reasons) = compute_deletion_safety_fields(0, 2, true);
        assert!(blocked);
        assert!(reasons.iter().any(|r| r.contains("未合并") && r.contains("2")));
    }

    #[test]
    fn compute_safety_ignores_unmerged_when_not_deleting_branch() {
        // 未托管：will_delete_branch=false，unmerged 不应阻断
        let (blocked, _reasons) = compute_deletion_safety_fields(0, 2, false);
        assert!(!blocked);
    }

    #[test]
    fn compute_safety_blocked_by_both_lists_both_reasons() {
        let (blocked, reasons) = compute_deletion_safety_fields(1, 1, true);
        assert!(blocked);
        assert_eq!(reasons.len(), 2);
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test compute_safety -- --nocapture`
Expected: 编译失败，`DeletionSafety` / `compute_deletion_safety_fields` 未定义。

- [ ] **Step 3: 写实现**

在 `RepoInfo` 结构体定义之后（`get_repo_info_cmd` 之前）插入：

```rust
/// 删除 worktree 前的安全预检结果
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

/// 纯逻辑：根据各项检查值计算 blocked 与 reasons。
/// - 未提交变更 > 0 → 阻断（无论是否托管）
/// - will_delete_branch 且 unmerged > 0 → 阻断
pub fn compute_deletion_safety_fields(
    uncommitted: u32,
    unmerged: u32,
    will_delete_branch: bool,
) -> (bool, Vec<String>) {
    let mut reasons = Vec::new();
    if uncommitted > 0 {
        reasons.push(format!("{} 个未提交变更", uncommitted));
    }
    if will_delete_branch && unmerged > 0 {
        reasons.push(format!("{} 个未合并到主干的提交", unmerged));
    }
    let blocked = !reasons.is_empty();
    (blocked, reasons)
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test compute_safety -- --nocapture && cargo test deletion_safety -- --nocapture`
Expected: 6 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands/worktree.rs
git commit -m "feat(worktree): add DeletionSafety struct + compute_deletion_safety_fields"
```

---

## Task 3: `preflight_delete_worktree_cmd` 命令

**Files:**
- Modify: `src-tauri/src/commands/worktree.rs`（在 `delete_worktree_cmd` 之后新增）

- [ ] **Step 1: 写实现**

在 `delete_worktree_cmd` 之后、`extract_name_from_path` 之前插入：

```rust
/// 删除 worktree 前的安全预检
#[tauri::command]
pub fn preflight_delete_worktree_cmd(
    path: String,
    repo_path: String,
    branch: Option<String>,
) -> Result<DeletionSafety, String> {
    info!("[preflight_delete_worktree_cmd] 开始: path={}, repo={}, branch={:?}",
          path, repo_path, branch);

    let repo = Path::new(&repo_path);
    let wt_path = Path::new(&path);

    // 1. 是否托管（DB 有记录）
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    let is_managed = get_worktree_by_path(&conn, &path)
        .map_err(|e| format!("数据库查询失败: {}", e))?
        .is_some();
    info!("[preflight_delete_worktree_cmd] is_managed={}", is_managed);

    // 2. 是否会删分支
    let will_delete_branch = is_managed
        && branch.as_ref().is_some_and(|b| branch_exists(repo, b));
    info!("[preflight_delete_worktree_cmd] will_delete_branch={}", will_delete_branch);

    // 3. 未提交变更（托管/未托管都查）
    let uncommitted_changes = match get_dirty_file_count(wt_path) {
        Ok(n) => n,
        Err(e) => {
            warn!("[preflight_delete_worktree_cmd] 获取未提交变更失败，按 0 处理: {}", e);
            0
        }
    };

    // 4. 未合并提交（仅 will_delete_branch 时查）
    let unmerged_commits = if will_delete_branch {
        if let Some(b) = &branch {
            match get_default_branch(repo) {
                Ok(main_branch) => match is_branch_merged(repo, b, &main_branch) {
                    Ok((_, n)) => n,
                    Err(e) => {
                        warn!("[preflight_delete_worktree_cmd] 合并检查失败，按 0 处理: {}", e);
                        0
                    }
                },
                Err(e) => {
                    warn!("[preflight_delete_worktree_cmd] 获取默认分支失败，按 0 处理: {}", e);
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    };

    let (blocked, reasons) =
        compute_deletion_safety_fields(uncommitted_changes, unmerged_commits, will_delete_branch);

    info!("[preflight_delete_worktree_cmd] 完成: uncommitted={}, unmerged={}, blocked={}, reasons={:?}",
          uncommitted_changes, unmerged_commits, blocked, reasons);

    Ok(DeletionSafety {
        is_managed,
        will_delete_branch,
        uncommitted_changes,
        unmerged_commits,
        blocked,
        reasons,
    })
}
```

- [ ] **Step 2: 修正 import**

在 `src-tauri/src/commands/worktree.rs` 顶部 `use crate::utils::git::{...}` 块中，把现有的：

```rust
use crate::utils::git::{
    RemoteInfo, get_repo_name, get_remotes, get_local_branches,
    get_remote_branches, get_default_branch, get_ahead_behind, get_dirty_file_count,
};
```

替换为：

```rust
use crate::utils::git::{
    RemoteInfo, get_repo_name, get_remotes, get_local_branches,
    get_remote_branches, get_default_branch, get_ahead_behind, get_dirty_file_count,
    branch_exists, is_branch_merged,
};
```

- [ ] **Step 3: 编译确认**

Run: `cd src-tauri && cargo build`
Expected: 编译通过，无 `unused import` 警告。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/commands/worktree.rs
git commit -m "feat(worktree): add preflight_delete_worktree_cmd safety check"
```

---

## Task 4: `count_live_worktrees` 纯函数 + `count_worktrees_cmd`

**Files:**
- Modify: `src-tauri/src/commands/worktree.rs`（新增函数与命令；测试加在 `mod tests` 内）

- [ ] **Step 1: 写失败测试**

在 `mod tests` 末尾追加（需引入 `GitWorktreeEntry`；测试通过构造 entries 验证计数）：

```rust
    use crate::utils::git::worktree::GitWorktreeEntry;

    #[test]
    fn count_live_excludes_main() {
        let entries = vec![
            GitWorktreeEntry { path: "/r".into(), head: "a".into(), branch: Some("main".into()), is_bare: false, is_main: true },
            GitWorktreeEntry { path: "/r.w/feat".into(), head: "b".into(), branch: Some("feat".into()), is_bare: false, is_main: false },
            GitWorktreeEntry { path: "/r.w/det".into(), head: "c".into(), branch: None, is_bare: false, is_main: false },
        ];
        assert_eq!(count_live_worktrees(&entries), 2);
    }

    #[test]
    fn count_live_zero_when_only_main() {
        let entries = vec![
            GitWorktreeEntry { path: "/r".into(), head: "a".into(), branch: Some("main".into()), is_bare: false, is_main: true },
        ];
        assert_eq!(count_live_worktrees(&entries), 0);
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test count_live -- --nocapture`
Expected: 编译失败，`count_live_worktrees` 未定义。

- [ ] **Step 3: 写实现**

在 `preflight_delete_worktree_cmd` 之后插入：

```rust
/// 纯逻辑：统计 live worktree 数（排除主仓库）
pub fn count_live_worktrees(entries: &[crate::utils::git::worktree::GitWorktreeEntry]) -> u32 {
    entries.iter().filter(|e| !e.is_main).count() as u32
}

/// 轻量计数：1 次 git porcelain + 1 次 DB 查询，供仓库折叠徽标使用
#[tauri::command]
pub fn count_worktrees_cmd(repo_path: String) -> Result<u32, String> {
    info!("[count_worktrees_cmd] 开始: repo={}", repo_path);

    let path = Path::new(&repo_path);

    // 1. live 计数
    let live = match list_worktrees_live(path) {
        Ok(entries) => count_live_worktrees(&entries),
        Err(e) => {
            warn!("[count_worktrees_cmd] 获取 live worktree 失败，按 0 处理: {}", e);
            0
        }
    };

    // 2. missing 计数（DB 有但 live 没有）
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    let db_items = list_worktrees_by_repo(&conn, &repo_path)
        .map_err(|e| format!("数据库查询失败: {}", e))?;
    let live_paths: std::collections::HashSet<String> = match list_worktrees_live(path) {
        Ok(entries) => entries.iter().map(|e| e.path.clone()).collect(),
        Err(_) => std::collections::HashSet::new(),
    };
    let missing = db_items.iter().filter(|d| !live_paths.contains(&d.path)).count() as u32;

    let total = live + missing;
    info!("[count_worktrees_cmd] 完成: live={}, missing={}, total={}", live, missing, total);
    Ok(total)
}
```

注意：`list_worktrees_live` 已在文件顶部 `use crate::utils::git::worktree::{...}` 中导入。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test count_live -- --nocapture`
Expected: 2 个测试 PASS。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/commands/worktree.rs
git commit -m "feat(worktree): add count_worktrees_cmd lightweight count"
```

---

## Task 5: 注册新命令到 `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs:26`（use 语句）与 `:168`（invoke_handler）

- [ ] **Step 1: 修改 use 语句**

将 `src-tauri/src/lib.rs:26`：

```rust
use commands::worktree::{create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd, delete_worktree_cmd};
```

改为：

```rust
use commands::worktree::{create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd, delete_worktree_cmd, preflight_delete_worktree_cmd, count_worktrees_cmd};
```

- [ ] **Step 2: 注册到 invoke_handler**

在 `delete_worktree_cmd,`（`lib.rs:168`）之后追加两行：

```rust
            delete_worktree_cmd,
            preflight_delete_worktree_cmd,
            count_worktrees_cmd,
```

- [ ] **Step 3: 编译 + 全量测试**

Run: `cd src-tauri && cargo build && cargo test`
Expected: 编译通过；全部测试 PASS（含新增 12 个）。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(worktree): register preflight + count commands"
```

---

## Task 6: 前端 `DeletionSafety` 类型

**Files:**
- Modify: `src/types/worktree.ts`

- [ ] **Step 1: 写实现**

在 `src/types/worktree.ts` 的 `WorktreeListItem` 接口之后追加：

```ts
export interface DeletionSafety {
  isManaged: boolean
  willDeleteBranch: boolean
  uncommittedChanges: number
  unmergedCommits: number
  blocked: boolean
  reasons: string[]
}
```

- [ ] **Step 2: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 3: 提交**

```bash
git add src/types/worktree.ts
git commit -m "feat(types): add DeletionSafety type"
```

---

## Task 7: 前端 api 调用

**Files:**
- Modify: `src/lib/api/worktrees.ts`

- [ ] **Step 1: 修改 import 与实现**

将 `src/lib/api/worktrees.ts:2` 的 import：

```ts
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo } from "@/types"
```

改为：

```ts
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo, DeletionSafety } from "@/types"
```

在 `deleteWorktree` 方法之后、`// Repo info` 注释之前插入两个方法：

```ts
  async preflightDeleteWorktree(
    path: string,
    repoPath: string,
    branch: string | null
  ): Promise<DeletionSafety> {
    return await invoke("preflight_delete_worktree_cmd", { path, repoPath, branch })
  },

  async countWorktrees(repoPath: string): Promise<number> {
    return await invoke("count_worktrees_cmd", { repoPath })
  },
```

- [ ] **Step 2: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 3: 提交**

```bash
git add src/lib/api/worktrees.ts
git commit -m "feat(api): add preflightDeleteWorktree + countWorktrees"
```

---

## Task 8: `useWorktreeCountQuery`

**Files:**
- Modify: `src/lib/query/worktreeQueries.ts`

- [ ] **Step 1: 写实现**

在 `useWorktreesQuery` 之后追加：

```ts
export const useWorktreeCountQuery = (repoPath: string | undefined) => {
  return useQuery<number>({
    queryKey: ["worktrees", "count", repoPath],
    queryFn: () => worktreesApi.countWorktrees(repoPath!),
    enabled: Boolean(repoPath),
    staleTime: 30 * 1000,
    refetchOnWindowFocus: false,
  })
}
```

- [ ] **Step 2: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 3: 提交**

```bash
git add src/lib/query/worktreeQueries.ts
git commit -m "feat(query): add useWorktreeCountQuery (always enabled, 30s stale)"
```

---

## Task 9: `RepoTreeItem` 徽标改读 count query

**Files:**
- Modify: `src/components/worktree/RepoTreeItem.tsx`

- [ ] **Step 1: 修改 import**

将 `RepoTreeItem.tsx:6`：

```ts
import { useWorktreesQuery } from "@/lib/query/worktreeQueries"
```

改为：

```ts
import { useWorktreesQuery, useWorktreeCountQuery } from "@/lib/query/worktreeQueries"
```

- [ ] **Step 2: 在组件内加 count query**

在 `RepoTreeItem.tsx` 的 `useWorktreesQuery(...)` 调用之后追加：

```ts
  const { data: count } = useWorktreeCountQuery(repoPath)
```

- [ ] **Step 3: 徽标改读 count**

将 `RepoTreeItem.tsx:56-58` 的徽标：

```tsx
        <span className="ml-auto text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
          {worktrees.length}
        </span>
```

改为：

```tsx
        <span className="ml-auto text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
          {count ?? 0}
        </span>
```

- [ ] **Step 4: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 5: 提交**

```bash
git add src/components/worktree/RepoTreeItem.tsx
git commit -m "fix(worktree): badge reads always-on count query instead of expanded-only list"
```

---

## Task 10: create/delete mutation 联动 invalidate count

**Files:**
- Modify: `src/lib/query/worktreeMutations.ts`

- [ ] **Step 1: create mutation `onSuccess` 增 invalidate**

在 `useCreateWorktreeMutation` 的 `onSuccess`（`worktreeMutations.ts:77`）的 `invalidateQueries` 之后追加一行，使该块变为：

```ts
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["worktrees", variables.repoPath] })
      queryClient.invalidateQueries({ queryKey: ["worktrees", "count", variables.repoPath] })
      toast.success(`Worktree "${_data.name}" 创建成功`)
    },
```

- [ ] **Step 2: delete mutation `onSuccess` 增 invalidate**

在 `useDeleteWorktreeMutation` 的 `onSuccess`（`worktreeMutations.ts:104`）的 `setQueryData` 之后追加一行，使该块变为：

```ts
    onSuccess: ({ path, repoPath }) => {
      queryClient.setQueryData<WorktreeListItem[]>(
        ["worktrees", repoPath],
        (current) => (current ?? []).filter((w) => w.path !== path)
      )
      queryClient.invalidateQueries({ queryKey: ["worktrees", "count", repoPath] })
      toast.success("Worktree 已删除")
    },
```

- [ ] **Step 3: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 4: 提交**

```bash
git add src/lib/query/worktreeMutations.ts
git commit -m "feat(worktree): invalidate count query on create/delete"
```

---

## Task 11: `DeleteWorktreeDialog` 三态对话框

**Files:**
- Create: `src/components/worktree/DeleteWorktreeDialog.tsx`
- Modify: `src/components/worktree/index.ts`

- [ ] **Step 1: 写组件**

创建 `src/components/worktree/DeleteWorktreeDialog.tsx`：

```tsx
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { AlertTriangle } from "lucide-react"
import type { DeletionSafety } from "@/types"

interface DeleteWorktreeDialogProps {
  open: boolean
  worktreeName: string
  branch: string | null
  safety: DeletionSafety | null
  onClose: () => void
  onConfirm: () => void
}

export function DeleteWorktreeDialog({
  open,
  worktreeName,
  branch,
  safety,
  onClose,
  onConfirm,
}: DeleteWorktreeDialogProps) {
  if (!safety) return null

  const blocked = safety.blocked
  const willDeleteBranch = safety.willDeleteBranch

  const description = willDeleteBranch
    ? `将删除 worktree「${worktreeName}」的目录和分支「${branch ?? "--"}」，此操作不可撤销。`
    : `将删除 worktree「${worktreeName}」的目录（未托管，不删除分支），此操作不可撤销。`

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className={blocked ? "text-destructive" : ""}>
            {blocked ? (
              <span className="flex items-center gap-1.5">
                <AlertTriangle className="w-4 h-4" />
                删除 Worktree - 存在风险
              </span>
            ) : (
              "删除 Worktree"
            )}
          </DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        {blocked && (
          <ul className="text-sm text-destructive space-y-1 my-2">
            {safety.reasons.map((r) => (
              <li key={r} className="flex items-center gap-1.5">
                <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                {r}
              </li>
            ))}
          </ul>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              onConfirm()
              onClose()
            }}
          >
            {blocked ? "我已知晓风险，强制删除" : "删除"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: 导出**

读取 `src/components/worktree/index.ts`（现有为 `export { WorktreeTab } from "./WorktreeTab"`），追加一行（双引号，匹配现有风格）：

```ts
export { DeleteWorktreeDialog } from "./DeleteWorktreeDialog"
```

- [ ] **Step 3: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 4: 提交**

```bash
git add src/components/worktree/DeleteWorktreeDialog.tsx src/components/worktree/index.ts
git commit -m "feat(worktree): add DeleteWorktreeDialog with risk + force-confirm states"
```

---

## Task 12: `WorktreeTab` 删除流程改预检

**Files:**
- Modify: `src/components/worktree/WorktreeTab.tsx`

- [ ] **Step 1: 修改 import**

将 `WorktreeTab.tsx:8` 的 `RepoTree` import 行下方，把现有 `ConfirmDialog` 引用替换。先把 import 块中的：

```ts
import { ConfirmDialog } from "@/components/dialogs"
```

改为（删除对话框改用专用组件）：

```ts
import { DeleteWorktreeDialog } from "./DeleteWorktreeDialog"
```

并在 `WorktreeTab.tsx:11` 的 query import 中追加 `useWorktreeCountQuery` 无需（本文件不用）。改为引入 `worktreesApi`：

将 `WorktreeTab.tsx:12-16` 的 mutations import 保持不变；在 `WorktreeTab.tsx:2` 的 `invoke` import 之后追加：

```ts
import { worktreesApi } from "@/lib/api/worktrees"
```

并引入类型 `DeletionSafety`：将 `WorktreeTab.tsx:18`：

```ts
import type { WorktreeListItem } from "@/types"
```

改为：

```ts
import type { WorktreeListItem, DeletionSafety } from "@/types"
```

- [ ] **Step 2: 扩展删除确认状态**

将 `WorktreeTab.tsx:31-35` 的 `deleteConfirm` 状态：

```ts
  const [deleteConfirm, setDeleteConfirm] = useState<{
    open: boolean
    worktree: WorktreeListItem | null
    deleteBranch: boolean
  }>({ open: false, worktree: null, deleteBranch: true })
```

改为：

```ts
  const [deleteConfirm, setDeleteConfirm] = useState<{
    open: boolean
    worktree: WorktreeListItem | null
    safety: DeletionSafety | null
  }>({ open: false, worktree: null, safety: null })
```

- [ ] **Step 3: 改 `handleDeleteWorktree` 为预检**

将 `WorktreeTab.tsx:77-79`：

```ts
  const handleDeleteWorktree = useCallback((worktree: WorktreeListItem) => {
    setDeleteConfirm({ open: true, worktree, deleteBranch: true })
  }, [])
```

改为：

```ts
  const handleDeleteWorktree = useCallback(async (worktree: WorktreeListItem) => {
    const repo = trackedRepos.find(
      (r) => wtPathStartsWith(worktree.path, r.path)
    )
    const repoPath = repo?.path ?? worktree.path
    try {
      const safety = await worktreesApi.preflightDeleteWorktree(
        worktree.path,
        repoPath,
        worktree.branch ?? null
      )
      setDeleteConfirm({ open: true, worktree, safety })
    } catch (e) {
      console.error("删除预检失败:", e)
      setDeleteConfirm({ open: true, worktree, safety: null })
    }
  }, [trackedRepos])
```

注意：`wtPathStartsWith` 是下面的辅助函数。在文件底部（组件之后）追加：

```ts
/** 兼容正反斜杠的路径前缀匹配 */
function wtPathStartsWith(childPath: string, parentPath: string): boolean {
  return (
    childPath.startsWith(parentPath + "/") ||
    childPath.startsWith(parentPath + "\\")
  )
}
```

- [ ] **Step 4: 改 `handleConfirmDelete` 用 `willDeleteBranch`**

将 `WorktreeTab.tsx:81-100`：

```ts
  const handleConfirmDelete = useCallback(() => {
    if (!deleteConfirm.worktree) return
    const wt = deleteConfirm.worktree
    // 从 trackedRepos 找到对应的 repoPath
    const repo = trackedRepos.find((r) => wt.path.startsWith(r.path + "/") || wt.path.startsWith(r.path + "\\"))
    const repoPath = repo?.path ?? wt.path

    // 立即清空详情面板（不等 mutation 返回）
    if (selectedWorktree?.path === wt.path) {
      setSelectedWorktree(null)
    }

    deleteWorktreeMutation.mutate({
      path: wt.path,
      repoPath,
      branch: wt.branch ?? null,
      deleteBranch: deleteConfirm.deleteBranch,
    })
    setDeleteConfirm({ open: false, worktree: null, deleteBranch: true })
  }, [deleteWorktreeMutation, deleteConfirm, selectedWorktree, trackedRepos])
```

改为：

```ts
  const handleConfirmDelete = useCallback(() => {
    if (!deleteConfirm.worktree) return
    const wt = deleteConfirm.worktree
    const repo = trackedRepos.find((r) => wtPathStartsWith(wt.path, r.path))
    const repoPath = repo?.path ?? wt.path
    const deleteBranch = deleteConfirm.safety?.willDeleteBranch ?? false

    // 立即清空详情面板（不等 mutation 返回）
    if (selectedWorktree?.path === wt.path) {
      setSelectedWorktree(null)
    }

    deleteWorktreeMutation.mutate({
      path: wt.path,
      repoPath,
      branch: wt.branch ?? null,
      deleteBranch,
    })
    setDeleteConfirm({ open: false, worktree: null, safety: null })
  }, [deleteWorktreeMutation, deleteConfirm, selectedWorktree, trackedRepos])
```

- [ ] **Step 5: 替换删除对话框 JSX**

将 `WorktreeTab.tsx:207-215` 的删除 `ConfirmDialog`：

```tsx
      <ConfirmDialog
        open={deleteConfirm.open}
        onClose={() => setDeleteConfirm({ open: false, worktree: null, deleteBranch: true })}
        onConfirm={handleConfirmDelete}
        title="删除 Worktree"
        description={`将删除 worktree「${deleteConfirm.worktree?.name ?? ""}」的目录和分支，此操作不可撤销。`}
        confirmText="删除"
        variant="destructive"
      />
```

改为：

```tsx
      <DeleteWorktreeDialog
        open={deleteConfirm.open}
        worktreeName={deleteConfirm.worktree?.name ?? ""}
        branch={deleteConfirm.worktree?.branch ?? null}
        safety={deleteConfirm.safety}
        onClose={() => setDeleteConfirm({ open: false, worktree: null, safety: null })}
        onConfirm={handleConfirmDelete}
      />
```

- [ ] **Step 6: 移除残留的 `ConfirmDialog` 用于删除的引用（若仍有他用则保留 import）**

`ConfirmDialog` 仍被「移除仓库」对话框使用（`WorktreeTab.tsx:196-204`），所以**不要**删除 `ConfirmDialog` 的 import——本任务 Step 1 已将 import 改为只保留 `DeleteWorktreeDialog`，需回退：保留两者 import。将 Step 1 中的 import 调整为：

```ts
import { ConfirmDialog } from "@/components/dialogs"
import { DeleteWorktreeDialog } from "./DeleteWorktreeDialog"
```

即 `ConfirmDialog` 与 `DeleteWorktreeDialog` 同时导入。

- [ ] **Step 7: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错；无 `ConfirmDialog` 未使用警告。

- [ ] **Step 8: 提交**

```bash
git add src/components/worktree/WorktreeTab.tsx
git commit -m "feat(worktree): delete flow uses preflight + DeleteWorktreeDialog"
```

---

## Task 13: 全量验证

- [ ] **Step 1: 后端测试**

Run: `cd src-tauri && cargo test`
Expected: 全部 PASS（含新增 `parse_rev_list_count`×4、`compute_safety`×5、`deletion_safety`×1、`count_live`×2 = 12 个）。

- [ ] **Step 2: 前端类型检查**

Run: `npx tsc --noEmit`
Expected: 无错。

- [ ] **Step 3: 手动验证（`npm run tauri dev`）**

按 spec「验证」节逐项确认：
1. 托管 worktree 有未提交变更 → 删除弹阻断框，点「强制删除」后目录+分支均消失。
2. 托管 worktree 分支未合并 → 阻断框；合并后普通删除 → 分支同步删除。
3. 未托管 worktree 删除 → 仅删目录，分支保留，对话框文案为「未托管，不删除分支」。
4. 仓库折叠时徽标显示正确计数；新建/删除 worktree 后计数即时刷新。

- [ ] **Step 4: 提交（如有手动修复）**

若手动验证发现需修补，修复后提交；否则跳过。

---

## Self-Review 记录

- **Spec 覆盖**：任务 1-5 覆盖任务 1 后端（`is_branch_merged`、`DeletionSafety`、`preflight`、注册）；任务 6-10、12 覆盖任务 1 前端 + 任务 2 前端；任务 4-5、8-10 覆盖任务 2 后端+前端计数。无遗漏。
- **占位符**：无 TBD/TODO；所有代码步骤均给出完整代码。
- **类型一致**：`DeletionSafety` 字段（`isManaged`/`willDeleteBranch`/`uncommittedChanges`/`unmergedCommits`/`blocked`/`reasons`）前后端一致；`compute_deletion_safety_fields` 签名贯穿任务 2/3 一致；`count_live_worktrees` 签名贯穿任务 4 一致；`preflightDeleteWorktree`/`countWorktrees` API 名一致。
