# Worktree UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Worktree management tab to Claude Fleet with repo tracking, worktree browsing, creation, and Claude Code launching.

**Architecture:** New "Worktree" tab (second position) with left-right split layout. Left sidebar: repo directory tree with expandable worktree lists. Right panel: worktree detail with info, git status placeholder, and action buttons. Backend adds `tracked_repos` table + CRUD. Frontend uses TanStack Query for data fetching.

**Tech Stack:** Tauri 2.0 (Rust backend), React + TypeScript, TanStack Query, Zustand, shadcn/ui, Tailwind CSS, lucide-react

**Spec:** `docs/superpowers/specs/2026-06-18-worktree-ui-design.md`

---

## File Structure

### Backend (Create)
- `src-tauri/src/db/tracked_repos.rs` — TrackedRepo struct + CRUD functions + Tauri commands + tests

### Backend (Modify)
- `src-tauri/src/db/schema.rs` — Add `tracked_repos` table to `init_tables()`
- `src-tauri/src/db/mod.rs` — Add `pub mod tracked_repos;`
- `src-tauri/src/lib.rs` — Import and register 3 new tracked_repos commands

### Frontend (Create)
- `src/lib/api/worktrees.ts` — API layer wrapping worktree + tracked_repos invoke calls
- `src/lib/query/worktreeQueries.ts` — TanStack Query hooks for worktree data
- `src/lib/query/worktreeMutations.ts` — TanStack Query mutations for worktree operations
- `src/components/worktree/WorktreeTab.tsx` — Main tab component (orchestrator)
- `src/components/worktree/RepoTree.tsx` — Left sidebar repo tree
- `src/components/worktree/RepoTreeItem.tsx` — Single repo row (expand/collapse + badge + delete)
- `src/components/worktree/WorktreeTreeItem.tsx` — Single worktree row (click to select)
- `src/components/worktree/WorktreeDetail.tsx` — Right panel detail view
- `src/components/worktree/CreateWorktreeDialog.tsx` — Smart form dialog
- `src/components/worktree/index.ts` — Barrel export

### Frontend (Modify)
- `src/types/worktree.ts` — Add `TrackedRepo` interface
- `src/types/index.ts` — Already re-exports worktree, no change needed
- `src/components/layout/AppLayout.tsx` — Add worktree tab entry
- `src/App.tsx` — Add worktree tab conditional render

---

## Task 1: Backend — tracked_repos Database Module

**Files:**
- Modify: `src-tauri/src/db/schema.rs` (add table in `init_tables()`)
- Modify: `src-tauri/src/db/mod.rs` (add module declaration)
- Create: `src-tauri/src/db/tracked_repos.rs` (struct + CRUD + commands + tests)

- [ ] **Step 1: Add tracked_repos table to schema**

In `src-tauri/src/db/schema.rs`, add the `tracked_repos` CREATE TABLE to the `execute_batch` call, after the `worktrees` table definition:

```rust
// Inside init_tables(), in the conn.execute_batch() string, append:
CREATE TABLE IF NOT EXISTS tracked_repos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    added_at INTEGER NOT NULL
);
```

- [ ] **Step 2: Add module declaration to db/mod.rs**

In `src-tauri/src/db/mod.rs`, add at the end:

```rust
pub mod tracked_repos;
```

- [ ] **Step 3: Create tracked_repos.rs with tests**

Create `src-tauri/src/db/tracked_repos.rs`:

```rust
// src-tauri/src/db/tracked_repos.rs
// 跟踪仓库管理

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::db::schema::get_connection;

/// 跟踪的仓库记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedRepo {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub added_at: i64,
}

/// 添加跟踪仓库。path 有 UNIQUE 约束，重复插入会报错。
pub fn add_tracked_repo(conn: &Connection, path: &str, name: &str) -> Result<TrackedRepo> {
    info!("[add_tracked_repo] 添加仓库: path={}, name={}", path, name);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    conn.execute(
        "INSERT INTO tracked_repos (path, name, added_at) VALUES (?1, ?2, ?3)",
        params![path, name, now],
    )?;

    let id = conn.last_insert_rowid();
    info!("[add_tracked_repo] 成功添加: id={}", id);

    Ok(TrackedRepo {
        id,
        path: path.to_string(),
        name: name.to_string(),
        added_at: now,
    })
}

/// 删除跟踪仓库
pub fn remove_tracked_repo(conn: &Connection, id: i64) -> Result<()> {
    info!("[remove_tracked_repo] 删除仓库: id={}", id);
    conn.execute("DELETE FROM tracked_repos WHERE id = ?1", params![id])?;
    info!("[remove_tracked_repo] 成功删除");
    Ok(())
}

/// 列出所有跟踪仓库
pub fn list_tracked_repos(conn: &Connection) -> Result<Vec<TrackedRepo>> {
    info!("[list_tracked_repos] 查询所有仓库");
    let mut stmt = conn.prepare(
        "SELECT id, path, name, added_at FROM tracked_repos ORDER BY added_at DESC"
    )?;

    let items = stmt.query_map([], |row| {
        Ok(TrackedRepo {
            id: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            added_at: row.get(3)?,
        })
    })?
    .collect::<Result<Vec<TrackedRepo>>>()?;

    info!("[list_tracked_repos] 共 {} 条记录", items.len());
    Ok(items)
}

// Tauri 命令

#[tauri::command]
pub fn add_tracked_repo_cmd(path: String, name: String) -> Result<TrackedRepo, String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    add_tracked_repo(&conn, &path, &name).map_err(|e| format!("添加仓库失败: {}", e))
}

#[tauri::command]
pub fn remove_tracked_repo_cmd(id: i64) -> Result<(), String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    remove_tracked_repo(&conn, id).map_err(|e| format!("删除仓库失败: {}", e))
}

#[tauri::command]
pub fn list_tracked_repos_cmd() -> Result<Vec<TrackedRepo>, String> {
    let conn = get_connection().map_err(|e| format!("数据库连接失败: {}", e))?;
    list_tracked_repos(&conn).map_err(|e| format!("查询仓库失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE tracked_repos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                added_at INTEGER NOT NULL
            );"
        ).expect("create table");
        conn
    }

    #[test]
    fn add_and_list() {
        let conn = setup_test_db();
        add_tracked_repo(&conn, "C:\\workspace\\project-a", "project-a").expect("add");
        add_tracked_repo(&conn, "C:\\workspace\\project-b", "project-b").expect("add");

        let repos = list_tracked_repos(&conn).expect("list");
        assert_eq!(repos.len(), 2);
        // added_at DESC: project-b first
        assert_eq!(repos[0].name, "project-b");
        assert_eq!(repos[1].name, "project-a");
    }

    #[test]
    fn duplicate_path_fails() {
        let conn = setup_test_db();
        add_tracked_repo(&conn, "C:\\workspace\\dup", "dup").expect("first add");
        let result = add_tracked_repo(&conn, "C:\\workspace\\dup", "dup2");
        assert!(result.is_err());
    }

    #[test]
    fn remove_deletes_record() {
        let conn = setup_test_db();
        let repo = add_tracked_repo(&conn, "C:\\workspace\\to-remove", "to-remove").expect("add");
        remove_tracked_repo(&conn, repo.id).expect("remove");

        let repos = list_tracked_repos(&conn).expect("list");
        assert!(repos.is_empty());
    }

    #[test]
    fn serde_camel_case_roundtrip() {
        let repo = TrackedRepo {
            id: 1,
            path: "C:\\test".to_string(),
            name: "test".to_string(),
            added_at: 1718668800,
        };
        let json = serde_json::to_string(&repo).expect("serialize");
        assert!(json.contains("addedAt"));
        assert!(!json.contains("added_at"));

        let parsed: TrackedRepo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.name, "test");
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cd src-tauri && cargo test tracked_repos -- --nocapture
```

Expected: 4 tests pass (add_and_list, duplicate_path_fails, remove_deletes_record, serde_camel_case_roundtrip).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/schema.rs src-tauri/src/db/mod.rs src-tauri/src/db/tracked_repos.rs
git commit -m "feat: add tracked_repos database module with CRUD"
```

---

## Task 2: Backend — Register tracked_repos Commands

**Files:**
- Modify: `src-tauri/src/lib.rs` (import + register)

- [ ] **Step 1: Add import in lib.rs**

In `src-tauri/src/lib.rs`, after the existing `use db::favorite_paths::...` line (around line 28), add:

```rust
use db::tracked_repos::{add_tracked_repo_cmd, remove_tracked_repo_cmd, list_tracked_repos_cmd};
```

- [ ] **Step 2: Register commands in invoke_handler**

In the `invoke_handler(tauri::generate_handler![...])` block, after the existing `// Worktree commands` section, add:

```rust
            // Tracked repos commands
            add_tracked_repo_cmd,
            remove_tracked_repo_cmd,
            list_tracked_repos_cmd,
```

- [ ] **Step 3: Verify compilation**

```bash
cd src-tauri && cargo check
```

Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register tracked_repos Tauri commands"
```

---

## Task 3: Frontend — Types and API Layer

**Files:**
- Modify: `src/types/worktree.ts` (add TrackedRepo interface)
- Create: `src/lib/api/worktrees.ts` (invoke wrappers)

- [ ] **Step 1: Add TrackedRepo type**

In `src/types/worktree.ts`, append at the end:

```typescript
export interface TrackedRepo {
  id: number
  path: string
  name: string
  addedAt: number
}
```

- [ ] **Step 2: Create worktrees API layer**

Create `src/lib/api/worktrees.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core"
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo } from "@/types"

export const worktreesApi = {
  // Tracked repos
  async listTrackedRepos(): Promise<TrackedRepo[]> {
    return await invoke("list_tracked_repos")
  },

  async addTrackedRepo(path: string, name: string): Promise<TrackedRepo> {
    return await invoke("add_tracked_repo", { path, name })
  },

  async removeTrackedRepo(id: number): Promise<void> {
    return await invoke("remove_tracked_repo", { id })
  },

  // Worktrees
  async listWorktrees(repoPath: string): Promise<WorktreeListItem[]> {
    return await invoke("list_worktrees_cmd", { repoPath })
  },

  async createWorktree(
    repoPath: string,
    name: string,
    branch: string,
    baseRef: string
  ): Promise<WorktreeInfo> {
    return await invoke("create_worktree_cmd", { repoPath, name, branch, baseRef })
  },

  // Repo info
  async getRepoInfo(repoPath: string): Promise<RepoInfo> {
    return await invoke("get_repo_info_cmd", { repoPath })
  },
}
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 4: Commit**

```bash
git add src/types/worktree.ts src/lib/api/worktrees.ts
git commit -m "feat: add TrackedRepo type and worktrees API layer"
```

---

## Task 4: Frontend — TanStack Query Hooks

**Files:**
- Create: `src/lib/query/worktreeQueries.ts` (queries)
- Create: `src/lib/query/worktreeMutations.ts` (mutations)

- [ ] **Step 1: Create query hooks**

Create `src/lib/query/worktreeQueries.ts`:

```typescript
import { useQuery } from "@tanstack/react-query"
import { worktreesApi } from "@/lib/api/worktrees"
import type { TrackedRepo, WorktreeListItem, RepoInfo } from "@/types"

export const useTrackedReposQuery = () => {
  return useQuery<TrackedRepo[]>({
    queryKey: ["trackedRepos"],
    queryFn: () => worktreesApi.listTrackedRepos(),
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })
}

export const useWorktreesQuery = (repoPath: string | undefined) => {
  return useQuery<WorktreeListItem[]>({
    queryKey: ["worktrees", repoPath],
    queryFn: () => worktreesApi.listWorktrees(repoPath!),
    enabled: Boolean(repoPath),
    staleTime: 30 * 1000,
  })
}

export const useRepoInfoQuery = (repoPath: string | undefined) => {
  return useQuery<RepoInfo>({
    queryKey: ["repoInfo", repoPath],
    queryFn: () => worktreesApi.getRepoInfo(repoPath!),
    enabled: Boolean(repoPath),
    staleTime: 5 * 60 * 1000,
  })
}
```

- [ ] **Step 2: Create mutation hooks**

Create `src/lib/query/worktreeMutations.ts`:

```typescript
import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { worktreesApi } from "@/lib/api/worktrees"
import type { TrackedRepo } from "@/types"

export const useAddTrackedRepoMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ path, name }: { path: string; name: string }) => {
      return await worktreesApi.addTrackedRepo(path, name)
    },
    onSuccess: (repo: TrackedRepo) => {
      queryClient.setQueryData<TrackedRepo[]>(["trackedRepos"], (current) =>
        [repo, ...(current ?? [])]
      )
      toast.success(`已添加仓库: ${repo.name}`)
    },
    onError: (error: Error) => {
      if (error.message.includes("UNIQUE constraint")) {
        toast.error("该仓库已在列表中")
      } else {
        toast.error(`添加仓库失败: ${error.message}`)
      }
    },
  })
}

export const useRemoveTrackedRepoMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (id: number) => {
      await worktreesApi.removeTrackedRepo(id)
      return id
    },
    onSuccess: (id) => {
      queryClient.setQueryData<TrackedRepo[]>(["trackedRepos"], (current) =>
        (current ?? []).filter((repo) => repo.id !== id)
      )
      // Remove cached worktrees for this repo
      queryClient.removeQueries({ queryKey: ["worktrees"] })
      toast.success("已从列表中移除仓库")
    },
    onError: (error: Error) => {
      toast.error(`移除仓库失败: ${error.message}`)
    },
  })
}

export const useCreateWorktreeMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      repoPath,
      name,
      branch,
      baseRef,
    }: {
      repoPath: string
      name: string
      branch: string
      baseRef: string
    }) => {
      return await worktreesApi.createWorktree(repoPath, name, branch, baseRef)
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["worktrees", variables.repoPath] })
      toast.success(`Worktree "${_data.name}" 创建成功`)
    },
    onError: (error: Error) => {
      toast.error(`创建 Worktree 失败: ${error.message}`)
    },
  })
}
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/query/worktreeQueries.ts src/lib/query/worktreeMutations.ts
git commit -m "feat: add TanStack Query hooks for worktree data"
```

---

## Task 5: Frontend — RepoTree Components (Left Sidebar)

**Files:**
- Create: `src/components/worktree/RepoTree.tsx`
- Create: `src/components/worktree/RepoTreeItem.tsx`
- Create: `src/components/worktree/WorktreeTreeItem.tsx`

- [ ] **Step 1: Create RepoTreeItem.tsx**

This is a single repo row with expand/collapse, count badge, delete button, and its own worktree query hook.

Create `src/components/worktree/RepoTreeItem.tsx`:

```tsx
import { useState } from "react"
import { ChevronDown, ChevronRight, Folder, X } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { WorktreeTreeItem } from "./WorktreeTreeItem"
import { useWorktreesQuery } from "@/lib/query/worktreeQueries"
import type { WorktreeListItem } from "@/types"

interface RepoTreeItemProps {
  repoName: string
  repoPath: string
  repoId: number
  selectedWorktreePath: string | null
  onSelectWorktree: (worktree: WorktreeListItem) => void
  onRemoveRepo: (repoId: number) => void
  onAddWorktree: (repoPath: string) => void
}

export function RepoTreeItem({
  repoName,
  repoPath,
  repoId,
  selectedWorktreePath,
  onSelectWorktree,
  onRemoveRepo,
  onAddWorktree,
}: RepoTreeItemProps) {
  const [expanded, setExpanded] = useState(false)
  const [hovered, setHovered] = useState(false)

  // Each RepoTreeItem owns its own worktree query, enabled only when expanded
  const { data: worktrees = [], isLoading: worktreesLoading } = useWorktreesQuery(
    expanded ? repoPath : undefined
  )

  return (
    <div>
      {/* Repo header */}
      <div
        className={cn(
          "flex items-center gap-1.5 px-2 py-1.5 rounded cursor-pointer text-sm",
          "hover:bg-accent/50 transition-colors",
          expanded && "bg-accent/30"
        )}
        onClick={() => setExpanded(!expanded)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        {expanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        )}
        <Folder className="w-4 h-4 text-violet-500 shrink-0" />
        <span className="truncate font-medium">{repoName}</span>
        <span className="ml-auto text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
          {worktrees.length}
        </span>
        {hovered && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 shrink-0 opacity-50 hover:opacity-100 hover:text-destructive"
            onClick={(e) => {
              e.stopPropagation()
              onRemoveRepo(repoId)
            }}
            title="移除仓库"
          >
            <X className="w-3 h-3" />
          </Button>
        )}
      </div>

      {/* Worktree children */}
      {expanded && (
        <div className="ml-5 mt-0.5 space-y-0.5">
          {worktreesLoading ? (
            <div className="text-xs text-muted-foreground px-2 py-1">加载中...</div>
          ) : worktrees.length === 0 ? (
            <div className="text-xs text-muted-foreground px-2 py-1">暂无 worktree</div>
          ) : (
            worktrees.map((wt) => (
              <WorktreeTreeItem
                key={wt.path}
                worktree={wt}
                isSelected={selectedWorktreePath === wt.path}
                onSelect={() => onSelectWorktree(wt)}
              />
            ))
          )}
          {/* Add worktree button */}
          <button
            className="w-full text-xs text-muted-foreground/60 hover:text-muted-foreground
                       border border-dashed border-muted-foreground/20 hover:border-muted-foreground/40
                       rounded px-2 py-1 mt-1 transition-colors"
            onClick={() => onAddWorktree(repoPath)}
          >
            ＋ 新建 worktree
          </button>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Create WorktreeTreeItem.tsx**

A single worktree row inside an expanded repo.

Create `src/components/worktree/WorktreeTreeItem.tsx`:

```tsx
import { GitBranch } from "lucide-react"
import { cn } from "@/lib/utils"
import type { WorktreeListItem } from "@/types"

interface WorktreeTreeItemProps {
  worktree: WorktreeListItem
  isSelected: boolean
  onSelect: () => void
}

export function WorktreeTreeItem({
  worktree,
  isSelected,
  onSelect,
}: WorktreeTreeItemProps) {
  const isMissing = worktree.status === "missing"

  return (
    <div
      className={cn(
        "flex items-center gap-1.5 px-2 py-1 rounded cursor-pointer text-sm transition-colors",
        isSelected
          ? "bg-violet-100 border border-violet-300"
          : "hover:bg-accent/50",
        isMissing && "opacity-50"
      )}
      onClick={onSelect}
    >
      <GitBranch className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
      <span className="truncate">{worktree.name}</span>
    </div>
  )
}
```

- [ ] **Step 3: Create RepoTree.tsx**

The left sidebar container that orchestrates repo items and the add repo button.

Create `src/components/worktree/RepoTree.tsx`:

```tsx
import { Plus } from "lucide-react"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { RepoTreeItem } from "./RepoTreeItem"
import type { TrackedRepo, WorktreeListItem } from "@/types"

interface RepoTreeProps {
  repos: TrackedRepo[]
  selectedWorktreePath: string | null
  onSelectWorktree: (worktree: WorktreeListItem) => void
  onAddRepo: () => void
  onRemoveRepo: (repoId: number) => void
  onAddWorktree: (repoPath: string) => void
}

export function RepoTree({
  repos,
  selectedWorktreePath,
  onSelectWorktree,
  onAddRepo,
  onRemoveRepo,
  onAddWorktree,
}: RepoTreeProps) {
  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          仓库列表
        </span>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={onAddRepo}
          title="添加仓库"
        >
          <Plus className="w-4 h-4" />
        </Button>
      </div>

      {/* Repo list */}
      <ScrollArea className="flex-1">
        <div className="p-2 space-y-1">
          {repos.map((repo) => (
            <RepoTreeItem
              key={repo.id}
              repoId={repo.id}
              repoName={repo.name}
              repoPath={repo.path}
              selectedWorktreePath={selectedWorktreePath}
              onSelectWorktree={onSelectWorktree}
              onRemoveRepo={onRemoveRepo}
              onAddWorktree={onAddWorktree}
            />
          ))}

          {/* Bottom add repo hint */}
          {repos.length === 0 && (
            <div
              className="text-center py-6 text-xs text-muted-foreground/50 border border-dashed border-muted-foreground/20 rounded cursor-pointer hover:text-muted-foreground/70 hover:border-muted-foreground/40 transition-colors"
              onClick={onAddRepo}
            >
              ＋ 添加仓库
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  )
}
```

- [ ] **Step 4: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/worktree/RepoTree.tsx src/components/worktree/RepoTreeItem.tsx src/components/worktree/WorktreeTreeItem.tsx
git commit -m "feat: add RepoTree components for left sidebar"
```

---

## Task 6: Frontend — WorktreeDetail Component (Right Panel)

**Files:**
- Create: `src/components/worktree/WorktreeDetail.tsx`

- [ ] **Step 1: Create WorktreeDetail.tsx**

The right panel showing worktree info, git status placeholder, and action buttons.

Create `src/components/worktree/WorktreeDetail.tsx`:

```tsx
import { Play, FolderOpen, Code, Trash2, ChevronRight } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import type { WorktreeListItem } from "@/types"

interface WorktreeDetailProps {
  worktree: WorktreeListItem | null
  onLaunchClaude: (worktree: WorktreeListItem) => void
  onOpenDirectory: (path: string) => void
  onOpenVSCode: (path: string) => void
  onDelete: (worktree: WorktreeListItem) => void
}

const statusConfig = {
  active: { label: "Active", className: "bg-green-100 text-green-700 border-green-200" },
  missing: { label: "Missing", className: "bg-red-100 text-red-700 border-red-200" },
  unmanaged: { label: "Unmanaged", className: "bg-yellow-100 text-yellow-700 border-yellow-200" },
}

export function WorktreeDetail({
  worktree,
  onLaunchClaude,
  onOpenDirectory,
  onOpenVSCode,
  onDelete,
}: WorktreeDetailProps) {
  if (!worktree) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <ChevronRight className="w-8 h-8 text-muted-foreground/30 mb-2" />
        <p className="text-sm">选择一个 worktree 查看详情</p>
        <p className="text-xs text-muted-foreground/60 mt-1">或点击「新建 Worktree」创建</p>
      </div>
    )
  }

  const isMissing = worktree.status === "missing"
  const status = statusConfig[worktree.status]

  const formatDate = (ts: number | null) => {
    if (!ts) return "--"
    return new Date(ts).toLocaleString("zh-CN", { hour12: false })
  }

  return (
    <div className="flex flex-col h-full overflow-y-auto p-4">
      {/* Title + status */}
      <div className="flex items-center gap-2 mb-4">
        <h2 className="text-lg font-semibold">{worktree.name}</h2>
        <Badge variant="outline" className={cn("text-xs", status.className)}>
          {status.label}
        </Badge>
      </div>

      {/* Basic info */}
      <div className="bg-muted/40 rounded-lg p-3 mb-4 space-y-2 text-sm">
        <InfoRow label="路径" value={worktree.path} mono />
        <InfoRow label="分支" value={worktree.branch ?? "--"} />
        <InfoRow label="基于" value={worktree.baseRef ?? "--"} />
        <InfoRow label="创建时间" value={formatDate(worktree.createdAt)} />
      </div>

      {/* Git status placeholder */}
      <div className="bg-muted/40 rounded-lg p-3 mb-4">
        <h3 className="text-sm font-medium mb-2">Git 状态</h3>
        <div className="flex gap-4 text-sm">
          <div className="flex items-center gap-1">
            <span className="text-muted-foreground">--</span>
            <span className="text-muted-foreground/60">ahead</span>
          </div>
          <div className="flex items-center gap-1">
            <span className="text-muted-foreground">--</span>
            <span className="text-muted-foreground/60">behind</span>
          </div>
          <div className="flex items-center gap-1">
            <span className="text-muted-foreground">--</span>
            <span className="text-muted-foreground/60">未提交变更</span>
          </div>
        </div>
        <p className="text-xs text-muted-foreground/50 mt-2">Git 状态功能将在后续版本中实现</p>
      </div>

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2 mt-auto pt-4 border-t">
        <Button
          variant="default"
          size="sm"
          disabled={isMissing}
          onClick={() => onLaunchClaude(worktree)}
          className="bg-violet-600 hover:bg-violet-700"
        >
          <Play className="w-4 h-4 mr-1" />
          运行 Claude Code
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={isMissing}
          onClick={() => onOpenDirectory(worktree.path)}
        >
          <FolderOpen className="w-4 h-4 mr-1" />
          打开目录
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={isMissing}
          onClick={() => onOpenVSCode(worktree.path)}
        >
          <Code className="w-4 h-4 mr-1" />
          VS Code
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled
          title="功能开发中"
          className="opacity-50"
        >
          <Trash2 className="w-4 h-4 mr-1" />
          删除
        </Button>
      </div>
    </div>
  )
}

function InfoRow({
  label,
  value,
  mono,
}: {
  label: string
  value: string
  mono?: boolean
}) {
  return (
    <div className="flex items-start gap-2">
      <span className="text-muted-foreground w-16 shrink-0">{label}</span>
      <span className={cn("truncate", mono && "font-mono text-xs")}>{value}</span>
    </div>
  )
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/worktree/WorktreeDetail.tsx
git commit -m "feat: add WorktreeDetail component for right panel"
```

---

## Task 7: Frontend — CreateWorktreeDialog Component

**Files:**
- Create: `src/components/worktree/CreateWorktreeDialog.tsx`

- [ ] **Step 1: Create the smart form dialog**

Create `src/components/worktree/CreateWorktreeDialog.tsx`:

```tsx
import { useState, useEffect } from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Loader2, ChevronDown, ChevronRight } from "lucide-react"
import { useRepoInfoQuery } from "@/lib/query/worktreeQueries"
import { useCreateWorktreeMutation } from "@/lib/query/worktreeMutations"
import type { WorktreeInfo } from "@/types"

interface CreateWorktreeDialogProps {
  open: boolean
  onClose: () => void
  repoPath: string
  onCreated?: (worktree: WorktreeInfo) => void
}

// Windows 路径非法字符
const ILLEGAL_CHARS = /[\\/:*?"<>|]/

export function CreateWorktreeDialog({
  open,
  onClose,
  repoPath,
  onCreated,
}: CreateWorktreeDialogProps) {
  const [name, setName] = useState("")
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [customBranch, setCustomBranch] = useState("")
  const [baseRef, setBaseRef] = useState("")

  const { data: repoInfo, isLoading: repoInfoLoading } = useRepoInfoQuery(
    open ? repoPath : undefined
  )
  const createMutation = useCreateWorktreeMutation()

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setName("")
      setShowAdvanced(false)
      setCustomBranch("")
      setBaseRef("")
    }
  }, [open])

  // Set default baseRef when repoInfo loads
  useEffect(() => {
    if (repoInfo && !baseRef) {
      const originDefault = `origin/${repoInfo.defaultBranch}`
      const hasOriginDefault = repoInfo.remoteBranches.includes(originDefault)
      setBaseRef(hasOriginDefault ? originDefault : repoInfo.defaultBranch)
    }
  }, [repoInfo, baseRef])

  const effectiveBranch = showAdvanced && customBranch.trim()
    ? customBranch.trim()
    : name.trim()
  const effectiveBaseRef = baseRef || "main"

  const handleCreate = async () => {
    if (!name.trim()) return

    try {
      const result = await createMutation.mutateAsync({
        repoPath,
        name: name.trim(),
        branch: effectiveBranch,
        baseRef: effectiveBaseRef,
      })
      onCreated?.(result)
      onClose()
    } catch {
      // Error handled by mutation's onError
    }
  }

  const nameError = name.trim()
    ? ILLEGAL_CHARS.test(name.trim())
      ? "名称包含非法字符 (\\ / : * ? \" < > |)"
      : null
    : null

  // Build branch options grouped by source
  const branchOptions = repoInfo
    ? [
        ...repoInfo.remoteBranches.map((b) => ({ value: b, label: b, group: "remote" })),
        ...repoInfo.localBranches
          .filter((b) => !repoInfo.remoteBranches.includes(b))
          .map((b) => ({ value: b, label: b, group: "local" })),
      ]
    : []

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>新建 Worktree</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-4">
          {/* Name input */}
          <div className="flex flex-col gap-2">
            <Label htmlFor="wt-name">Worktree 名称</Label>
            <Input
              id="wt-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如: feature-auth"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && name.trim() && !nameError) {
                  handleCreate()
                }
              }}
            />
            {nameError && (
              <p className="text-xs text-destructive">{nameError}</p>
            )}
          </div>

          {/* Auto-config summary */}
          {name.trim() && (
            <div className="bg-violet-50 border border-violet-200 rounded-md px-3 py-2 text-sm text-violet-700">
              分支：<span className="font-medium">{effectiveBranch}</span>
              {" · "}基于：<span className="font-medium">{effectiveBaseRef}</span>
            </div>
          )}

          {/* Advanced toggle */}
          <button
            type="button"
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            {showAdvanced ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
            高级选项
          </button>

          {/* Advanced options */}
          {showAdvanced && (
            <div className="flex flex-col gap-3 pl-5 border-l-2 border-muted">
              <div className="flex flex-col gap-2">
                <Label htmlFor="wt-branch" className="text-sm">
                  分支名 <span className="text-muted-foreground font-normal">(留空则同名称)</span>
                </Label>
                <Input
                  id="wt-branch"
                  value={customBranch}
                  onChange={(e) => setCustomBranch(e.target.value)}
                  placeholder="自动"
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label className="text-sm">基于分支 / ref</Label>
                {repoInfoLoading ? (
                  <div className="text-sm text-muted-foreground flex items-center gap-2">
                    <Loader2 className="w-3 h-3 animate-spin" />
                    加载分支列表...
                  </div>
                ) : (
                  <Select value={baseRef} onValueChange={setBaseRef}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="选择基准分支" />
                    </SelectTrigger>
                    <SelectContent>
                      {branchOptions.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                          {opt.group === "remote" && (
                            <span className="ml-2 text-xs text-muted-foreground">remote</span>
                          )}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button
            variant="default"
            onClick={handleCreate}
            disabled={!name.trim() || !!nameError || createMutation.isPending}
            className="bg-violet-600 hover:bg-violet-700"
          >
            {createMutation.isPending ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                创建中...
              </>
            ) : (
              "创建"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/worktree/CreateWorktreeDialog.tsx
git commit -m "feat: add CreateWorktreeDialog with smart form"
```

---

## Task 8: Frontend — WorktreeTab Main Component

**Files:**
- Create: `src/components/worktree/WorktreeTab.tsx`
- Create: `src/components/worktree/index.ts`

- [ ] **Step 1: Create WorktreeTab.tsx**

The main orchestrator component that wires together all sub-components.

Create `src/components/worktree/WorktreeTab.tsx`:

```tsx
import { useState, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { open as openDialog } from "@tauri-apps/plugin-dialog"
import { Plus, RefreshCw } from "lucide-react"
import { useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/dialogs"
import { RepoTree } from "./RepoTree"
import { WorktreeDetail } from "./WorktreeDetail"
import { CreateWorktreeDialog } from "./CreateWorktreeDialog"
import { useTrackedReposQuery } from "@/lib/query/worktreeQueries"
import {
  useAddTrackedRepoMutation,
  useRemoveTrackedRepoMutation,
} from "@/lib/query/worktreeMutations"
import { startNewSession } from "@/services/sessionLaunchService"
import type { WorktreeListItem } from "@/types"

export function WorktreeTab() {
  const queryClient = useQueryClient()

  // State
  const [selectedWorktree, setSelectedWorktree] = useState<WorktreeListItem | null>(null)
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [createDialogRepoPath, setCreateDialogRepoPath] = useState("")
  const [removeRepoConfirm, setRemoveRepoConfirm] = useState<{
    open: boolean
    repoId: number
    repoName: string
  }>({ open: false, repoId: 0, repoName: "" })

  // Queries
  const { data: trackedRepos = [] } = useTrackedReposQuery()

  // Mutations
  const addRepoMutation = useAddTrackedRepoMutation()
  const removeRepoMutation = useRemoveTrackedRepoMutation()

  // Handlers
  const handleAddRepo = useCallback(async () => {
    try {
      const selected = await openDialog({ directory: true, multiple: false })
      if (!selected) return

      const path = selected as string
      // Extract name from path
      const parts = path.split(/[\\/]/).filter(Boolean)
      const name = parts.pop() || "unknown"

      addRepoMutation.mutate({ path, name })
    } catch (e) {
      console.error("添加仓库失败:", e)
    }
  }, [addRepoMutation])

  const handleRemoveRepo = useCallback((repoId: number) => {
    const repo = trackedRepos.find((r) => r.id === repoId)
    if (repo) {
      setRemoveRepoConfirm({ open: true, repoId, repoName: repo.name })
    }
  }, [trackedRepos])

  const handleConfirmRemoveRepo = useCallback(() => {
    removeRepoMutation.mutate(removeRepoConfirm.repoId)
    // Clear selection if the selected worktree belongs to the removed repo
    if (selectedWorktree) {
      const removedRepo = trackedRepos.find((r) => r.id === removeRepoConfirm.repoId)
      if (removedRepo && selectedWorktree.repoName === removedRepo.name) {
        setSelectedWorktree(null)
      }
    }
    setRemoveRepoConfirm({ open: false, repoId: 0, repoName: "" })
  }, [removeRepoMutation, removeRepoConfirm, selectedWorktree, trackedRepos])

  const handleAddWorktree = useCallback((repoPath: string) => {
    setCreateDialogRepoPath(repoPath)
    setCreateDialogOpen(true)
  }, [])

  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ["worktrees"] })
  }, [queryClient])

  const handleLaunchClaude = useCallback(async (worktree: WorktreeListItem) => {
    try {
      await startNewSession({
        workingDirectory: worktree.path,
        name: worktree.name,
      })
    } catch (e) {
      console.error("启动 Claude Code 失败:", e)
    }
  }, [])

  const handleOpenDirectory = useCallback((path: string) => {
    invoke("open_directory", { path }).catch(console.error)
  }, [])

  const handleOpenVSCode = useCallback((path: string) => {
    invoke("open_in_vscode", { path }).catch(console.error)
  }, [])

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b bg-white">
        <h2 className="text-base font-semibold text-gray-900 shrink-0">Worktree</h2>
        <div className="w-px h-6 bg-gray-200" />
        <Button
          variant="default"
          size="sm"
          onClick={() => {
            if (trackedRepos.length > 0) {
              setCreateDialogRepoPath(trackedRepos[0].path)
              setCreateDialogOpen(true)
            } else {
              handleAddRepo()
            }
          }}
          className="h-8 bg-violet-600 hover:bg-violet-700"
        >
          <Plus className="w-4 h-4 mr-1" />
          新建 Worktree
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={handleRefresh}
          className="h-8"
          title="刷新"
        >
          <RefreshCw className="w-4 h-4" />
        </Button>
      </div>

      {/* Split layout */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Left sidebar */}
        <div className="w-[240px] min-w-[240px] border-r border-gray-200 bg-gray-50 overflow-hidden">
          <RepoTree
            repos={trackedRepos}
            selectedWorktreePath={selectedWorktree?.path ?? null}
            onSelectWorktree={setSelectedWorktree}
            onAddRepo={handleAddRepo}
            onRemoveRepo={handleRemoveRepo}
            onAddWorktree={handleAddWorktree}
          />
        </div>

        {/* Right detail panel */}
        <div className="flex-1 min-w-0 bg-white overflow-hidden">
          <WorktreeDetail
            worktree={selectedWorktree}
            onLaunchClaude={handleLaunchClaude}
            onOpenDirectory={handleOpenDirectory}
            onOpenVSCode={handleOpenVSCode}
            onDelete={() => {}}
          />
        </div>
      </div>

      {/* Create worktree dialog */}
      <CreateWorktreeDialog
        open={createDialogOpen}
        onClose={() => setCreateDialogOpen(false)}
        repoPath={createDialogRepoPath}
        onCreated={() => {
          // Query invalidation handled by mutation's onSuccess
        }}
      />

      {/* Remove repo confirmation */}
      <ConfirmDialog
        open={removeRepoConfirm.open}
        onClose={() => setRemoveRepoConfirm({ open: false, repoId: 0, repoName: "" })}
        onConfirm={handleConfirmRemoveRepo}
        title="移除仓库"
        description={`将从列表中移除「${removeRepoConfirm.repoName}」，不会删除本地文件。`}
        confirmText="移除"
        variant="destructive"
      />
    </div>
  )
}
```

- [ ] **Step 2: Create barrel export**

Create `src/components/worktree/index.ts`:

```typescript
export { WorktreeTab } from "./WorktreeTab"
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/worktree/
git commit -m "feat: add WorktreeTab main component"
```

---

## Task 9: Frontend — Wire Tab into AppLayout and App

**Files:**
- Modify: `src/components/layout/AppLayout.tsx` (add tab)
- Modify: `src/App.tsx` (add render)

- [ ] **Step 1: Add Worktree tab to AppLayout**

In `src/components/layout/AppLayout.tsx`, update the TABS array:

```typescript
const TABS = [
  { id: "running", label: "运行中" },
  { id: "worktree", label: "Worktree" },
  { id: "management", label: "Session 管理" },
]
```

- [ ] **Step 2: Add Worktree render to App.tsx**

In `src/App.tsx`:

1. Add import at the top:
```typescript
import { WorktreeTab } from "@/components/worktree"
```

2. Add conditional render inside `<AppLayout>`:
```tsx
{activeTab === "worktree" && <WorktreeTab />}
```

The render block should now be:
```tsx
<AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
  {activeTab === "running" && <RunningTab />}
  {activeTab === "worktree" && <WorktreeTab />}
  {activeTab === "management" && <ManagementTab />}
</AppLayout>
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 4: Full build verification**

```bash
npm run build
```

Expected: Build succeeds with no errors.

- [ ] **Step 5: Run the app and verify**

```bash
npm run tauri dev
```

Verify:
1. Three tabs appear: 运行中 → Worktree → Session 管理
2. Clicking "Worktree" shows the split layout
3. "添加仓库" button opens folder picker
4. After adding a repo, it appears in the left sidebar
5. Expanding a repo shows worktree list (if any exist)
6. Clicking a worktree shows details in the right panel
7. "新建 Worktree" opens the smart form dialog
8. "运行 Claude Code" launches a session in the worktree path
9. "打开目录" opens Windows Explorer
10. "VS Code" opens VS Code
11. Delete button is disabled with "功能开发中" tooltip
12. Git status section shows "--" placeholders

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat: wire Worktree tab into app navigation"
```
