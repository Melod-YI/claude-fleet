# Worktree 刷新按钮接入 git fetch 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让新建 Worktree 对话框的刷新按钮在重新读取分支列表前先执行 `git fetch --all --prune`（30s 超时），失败时降级展示本地缓存并给出红点提示。

**Architecture:** 新增独立后端命令 `fetch_repo_remotes_cmd`（封装带超时的 `fetch_remotes` 工具函数，返回结构化 `FetchResult`），前端刷新按钮先调该命令、再 `refetchRepoInfo()`，据 `FetchResult.success` 决定是否显示失败提示。fetch 与 repoInfo 职责分离，不污染既有 `RepoInfo` 类型。

**Tech Stack:** Rust + Tauri 2.0（`std::process` spawn/try_wait、`crate::utils::process::command`）、React + TypeScript + TanStack Query。

参考设计：`docs/superpowers/specs/2026-06-27-worktree-refresh-fetch-design.md`

---

## 文件结构

| 文件 | 责任 | 改动 |
|---|---|---|
| `src-tauri/src/utils/git/mod.rs` | 通用 git 封装 | 新增 `fetch_remotes` 函数；将 `test_helpers::git` 改为 `pub`；新增 2 个测试 |
| `src-tauri/src/commands/worktree.rs` | worktree Tauri 命令 | 新增 `FetchResult` 结构 + `fetch_repo_remotes_cmd` 命令 + 1 个 serde 测试 |
| `src-tauri/src/lib.rs` | 命令注册 | 注册 `fetch_repo_remotes_cmd` |
| `src/types/worktree.ts` | 前端类型 | 新增 `FetchResult` |
| `src/lib/api/worktrees.ts` | invoke 封装 | 新增 `fetchRepoRemotes` |
| `src/lib/query/worktreeMutations.ts` | TanStack mutation | 新增 `useFetchRepoRemotesMutation` |
| `src/components/worktree/CreateWorktreeDialog.tsx` | 对话框 UI | 刷新按钮 handler + 失败提示 UI + 状态 |

> 测试落位说明（对 spec 的细化）：`FetchResult` serde 测试放在 `commands/worktree.rs`（与类型同文件）；两个 `fetch_remotes` IO 测试放在 `utils/git/mod.rs` 的 `tests` 模块（与被测函数同文件，且可复用 `test_helpers`）。

---

## Task 1: 后端 `fetch_remotes` 工具函数（TDD）

**Files:**
- Modify: `src-tauri/src/utils/git/mod.rs`（顶部 imports + 新增 `fetch_remotes`；`test_helpers::git` 改 `pub`；`tests` 模块新增 2 测试）

- [ ] **Step 1: 在 `tests` 模块写失败测试 `fetch_remotes_returns_err_on_non_git_dir`**

在 `src-tauri/src/utils/git/mod.rs` 的 `mod tests { ... }` 内（`use super::*;` 之后）追加：

```rust
    #[test]
    fn fetch_remotes_returns_err_on_non_git_dir() {
        let dir = test_helpers::unique_temp_dir("fetch-nongit");
        let result = fetch_remotes(&dir, 30);
        assert!(result.is_err(), "非 git 目录应返回错误");
        assert!(!result.unwrap_err().is_empty(), "错误消息不应为空");
        let _ = std::fs::remove_dir_all(&dir);
    }
```

- [ ] **Step 2: 运行测试，确认失败（`fetch_remotes` 未定义）**

Run: `cd src-tauri && cargo test --lib fetch_remotes_returns_err_on_non_git_dir`
Expected: 编译错误 `cannot find function fetch_remotes`

- [ ] **Step 3: 添加 imports 并实现 `fetch_remotes`**

在文件顶部现有 imports（`use serde::...; use std::path::...; use tracing::...;`）之后追加：

```rust
use std::io::Read;
use std::process::Stdio;
use std::time::{Duration, Instant};
```

在 `get_remote_branches` 函数之后（`get_default_branch` 之前或之后均可，建议紧邻 `get_remote_branches` 之后）新增：

```rust
/// 拉取所有远端仓库（git fetch --all --prune），带超时。
/// 成功返回 Ok(())，失败/超时返回 Err(message)。
/// 独立于 execute_git：后者用 .output() 阻塞，无法施加超时。
pub fn fetch_remotes(repo_path: &Path, timeout_secs: u64) -> Result<(), String> {
    info!("[fetch_remotes] 开始: repo={}, timeout={}s", repo_path.display(), timeout_secs);

    let mut child = crate::utils::process::command("git")
        .arg("-C")
        .arg(repo_path)
        .args(["fetch", "--all", "--prune"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动 git fetch: {}", e))?;

    // 线程排空 stderr，避免管道写满导致子进程阻塞死锁
    let stderr_pipe = child.stderr.take();
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut s) = stderr_pipe {
            let _ = s.read_to_string(&mut buf);
        }
        buf
    });

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stderr_thread.join();
                    let msg = format!("git fetch 超时（{}s）", timeout_secs);
                    warn!("[fetch_remotes] {}", msg);
                    return Err(msg);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                let _ = stderr_thread.join();
                return Err(format!("等待 git fetch 进程失败: {}", e));
            }
        }
    };

    let stderr = stderr_thread.join().unwrap_or_default();
    if status.success() {
        info!("[fetch_remotes] 完成");
        Ok(())
    } else {
        let stderr = stderr.trim();
        let msg = if stderr.is_empty() {
            format!("git fetch 失败（exit {}）", status.code().unwrap_or(-1))
        } else {
            format!("git fetch 失败: {}", stderr)
        };
        warn!("[fetch_remotes] {}", msg);
        Err(msg)
    }
}
```

- [ ] **Step 4: 运行测试，确认通过**

Run: `cd src-tauri && cargo test --lib fetch_remotes_returns_err_on_non_git_dir`
Expected: PASS

- [ ] **Step 5: 将 `test_helpers::git` 改为 `pub`，供下一个测试复用**

在 `src-tauri/src/utils/git/mod.rs` 的 `test_helpers` 模块内，把：

```rust
    fn git(path: &Path, args: &[&str]) {
```

改为：

```rust
    pub fn git(path: &Path, args: &[&str]) {
```

- [ ] **Step 6: 写 happy path 测试 `fetch_remotes_success_against_local_bare`**

在 `mod tests` 内追加（用本地 bare 仓库作 file:// 远端，无网络依赖）：

```rust
    #[test]
    fn fetch_remotes_success_against_local_bare() {
        use crate::utils::git::test_helpers::{git, init_repo, unique_temp_path};

        // 1. bare 远端（路径不存在，由 git init --bare 创建）
        let remote = unique_temp_path("fetch-bare");
        let status = crate::utils::process::command("git")
            .arg("init")
            .arg("--bare")
            .arg(&remote)
            .status()
            .expect("git init --bare 执行失败");
        assert!(status.success(), "git init --bare 失败");

        // 2. 工作仓库（init_repo 已含 config 与初始提交）
        let work = init_repo("fetch-work");
        git(&work, &["branch", "-M", "main"]);
        let remote_str = remote.to_string_lossy().to_string();
        git(&work, &["remote", "add", "origin", remote_str.as_str()]);
        git(&work, &["push", "origin", "main"]);

        // 3. 删除远程跟踪引用，使 fetch 有实际工作可做
        git(&work, &["update-ref", "-d", "refs/remotes/origin/main"]);

        // 4. fetch 应成功
        let result = fetch_remotes(&work, 30);
        assert!(result.is_ok(), "fetch 应成功，实际: {:?}", result.err());

        // 5. 远程分支列表应包含 origin/main
        let remote_branches = get_remote_branches(&work).expect("读取远程分支失败");
        assert!(
            remote_branches.iter().any(|b| b == "origin/main"),
            "fetch 后应能看到 origin/main，实际: {:?}",
            remote_branches
        );

        let _ = std::fs::remove_dir_all(&work);
        let _ = std::fs::remove_dir_all(&remote);
    }
```

- [ ] **Step 7: 运行全部 git 模块测试，确认通过**

Run: `cd src-tauri && cargo test --lib -- git::`
Expected: 所有 `utils::git::tests::*` 通过（含两个新测试）

- [ ] **Step 8: 提交**

```bash
cd src-tauri && git add src/utils/git/mod.rs && git commit -m "feat(git): 新增带超时的 fetch_remotes 工具函数"
```

---

## Task 2: 后端 `FetchResult` 类型与 `fetch_repo_remotes_cmd` 命令（TDD）

**Files:**
- Modify: `src-tauri/src/commands/worktree.rs`（新增 `FetchResult` + 命令 + serde 测试）
- Modify: `src-tauri/src/lib.rs`（注册命令）

- [ ] **Step 1: 写失败测试 `fetch_result_camel_case_roundtrip`**

在 `src-tauri/src/commands/worktree.rs` 的 `#[cfg(test)]` 模块内追加：

```rust
    #[test]
    fn fetch_result_camel_case_roundtrip() {
        // 成功变体
        let ok = FetchResult { success: true, message: None };
        let json = serde_json::to_string(&ok).unwrap();
        assert!(json.contains("\"success\":true"));
        let parsed: FetchResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert!(parsed.message.is_none());

        // 失败变体
        let fail = FetchResult {
            success: false,
            message: Some("git fetch 失败: timeout".to_string()),
        };
        let json = serde_json::to_string(&fail).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"message\":\"git fetch 失败: timeout\""));
        let parsed: FetchResult = serde_json::from_str(&json).unwrap();
        assert!(!parsed.success);
        assert_eq!(parsed.message.unwrap(), "git fetch 失败: timeout");
    }
```

- [ ] **Step 2: 运行测试，确认失败（`FetchResult` 未定义）**

Run: `cd src-tauri && cargo test --lib fetch_result_camel_case_roundtrip`
Expected: 编译错误 `cannot find type FetchResult`

- [ ] **Step 3: 新增 `FetchResult` 结构**

在 `commands/worktree.rs` 中 `RepoInfo` 结构定义之后追加：

```rust
/// fetch 远端仓库的结果（前端用于决定是否显示失败提示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResult {
    pub success: bool,
    pub message: Option<String>,
}
```

- [ ] **Step 4: 运行测试，确认通过**

Run: `cd src-tauri && cargo test --lib fetch_result_camel_case_roundtrip`
Expected: PASS

- [ ] **Step 5: 新增 `fetch_repo_remotes_cmd` 命令**

在 `commands/worktree.rs` 中 `get_repo_info_cmd` 函数之后追加：

```rust
/// 拉取远端仓库（git fetch --all --prune，30s 超时）。
/// fetch 失败/超时时仍返回 Ok(FetchResult { success: false })，
/// 供前端降级展示本地缓存分支；仅命令本身无法执行才返回 Err。
#[tauri::command]
pub fn fetch_repo_remotes_cmd(repo_path: String) -> Result<FetchResult, String> {
    info!("[fetch_repo_remotes_cmd] 开始: repo={}", repo_path);
    let path = Path::new(&repo_path);
    match crate::utils::git::fetch_remotes(path, 30) {
        Ok(()) => {
            info!("[fetch_repo_remotes_cmd] 完成: 成功");
            Ok(FetchResult { success: true, message: None })
        }
        Err(e) => {
            warn!("[fetch_repo_remotes_cmd] fetch 失败，降级为本地缓存: {}", e);
            Ok(FetchResult { success: false, message: Some(e) })
        }
    }
}
```

> 注：`Path`、`info!`、`warn!`、`Serialize`/`Deserialize` 在本文件已导入（`get_repo_info_cmd` 已使用），无需新增 import。

- [ ] **Step 6: 在 `lib.rs` 注册命令**

在 `src-tauri/src/lib.rs` 第 27 行的 use 语句中，把 `fetch_repo_remotes_cmd` 加入 `commands::worktree::{...}` 导入：

```rust
use commands::worktree::{create_worktree_cmd, list_worktrees_cmd, get_repo_info_cmd, fetch_repo_remotes_cmd, delete_worktree_cmd, preflight_delete_worktree_cmd, count_worktrees_cmd};
```

在 `tauri::generate_handler!([...])` 列表中，`get_repo_info_cmd,` 之后追加一行 `fetch_repo_remotes_cmd,`：

```rust
            get_repo_info_cmd,
            fetch_repo_remotes_cmd,
            delete_worktree_cmd,
```

- [ ] **Step 7: 编译并跑全部 worktree 命令测试**

Run: `cd src-tauri && cargo test --lib -- commands::worktree`
Expected: 全部通过（含新 serde 测试），无编译错误

- [ ] **Step 8: 提交**

```bash
cd src-tauri && git add src/commands/worktree.rs src/lib.rs && git commit -m "feat(worktree): 新增 fetch_repo_remotes_cmd 命令"
```

---

## Task 3: 前端类型、API 与 mutation

**Files:**
- Modify: `src/types/worktree.ts`
- Modify: `src/lib/api/worktrees.ts`
- Modify: `src/lib/query/worktreeMutations.ts`

- [ ] **Step 1: 新增 `FetchResult` 类型**

在 `src/types/worktree.ts` 的 `RepoInfo` 接口之后追加：

```ts
export interface FetchResult {
  success: boolean
  message: string | null
}
```

> `src/types/index.ts` 已 `export * from './worktree'`，无需改动即可从 `@/types` 导出。

- [ ] **Step 2: 新增 `fetchRepoRemotes` API**

在 `src/lib/api/worktrees.ts` 的 `getRepoInfo` 方法之后追加（在 `getRepoInfo(repoPath: string)` 后、对象闭合 `}` 前）：

```ts
  // Repo info
  async getRepoInfo(repoPath: string): Promise<RepoInfo> {
    return await invoke("get_repo_info_cmd", { repoPath })
  },

  async fetchRepoRemotes(repoPath: string): Promise<FetchResult> {
    return await invoke("fetch_repo_remotes_cmd", { repoPath })
  },
```

并确认顶部 import 已含 `FetchResult`。当前 import 行为：

```ts
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo, DeletionSafety } from "@/types"
```

改为：

```ts
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo, FetchResult, DeletionSafety } from "@/types"
```

- [ ] **Step 3: 新增 `useFetchRepoRemotesMutation`**

在 `src/lib/query/worktreeMutations.ts` 文件末尾追加：

```ts
export const useFetchRepoRemotesMutation = () => {
  return useMutation<FetchResult, Error, string>({
    mutationFn: (repoPath: string) => worktreesApi.fetchRepoRemotes(repoPath),
  })
}
```

并在顶部 import 中加入 `FetchResult` 类型。当前：

```ts
import type { TrackedRepo } from "@/types"
```

改为：

```ts
import type { TrackedRepo, FetchResult } from "@/types"
```

- [ ] **Step 4: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误

- [ ] **Step 5: 提交**

```bash
git add src/types/worktree.ts src/lib/api/worktrees.ts src/lib/query/worktreeMutations.ts && git commit -m "feat(worktree): 新增 fetchRepoRemotes API 与 mutation"
```

---

## Task 4: 前端对话框刷新按钮接线

**Files:**
- Modify: `src/components/worktree/CreateWorktreeDialog.tsx`

- [ ] **Step 1: 新增 import 与状态**

在 `CreateWorktreeDialog.tsx` 顶部 import 区，`useCreateWorktreeMutation` 导入行之后追加：

```ts
import { useFetchRepoRemotesMutation } from "@/lib/query/worktreeMutations"
```

在组件函数体内（`const createMutation = useCreateWorktreeMutation()` 之后）追加：

```ts
  const fetchMutation = useFetchRepoRemotesMutation()
  const [fetchError, setFetchError] = useState<string | null>(null)
```

- [ ] **Step 2: 新增 `handleRefresh`**

在 `handleCreate` 函数之前新增：

```ts
  const handleRefresh = async () => {
    setFetchError(null)
    try {
      const res = await fetchMutation.mutateAsync(repoPath)
      // 无论 fetch 成功失败都刷新本地分支视图（失败时展示本地缓存）
      await refetchRepoInfo()
      if (!res.success && res.message) {
        setFetchError(res.message)
      }
    } catch {
      // invoke 级传输错误：不刷新列表，静默处理
    }
  }
```

- [ ] **Step 3: 对话框关闭时清空 `fetchError`**

在 `useEffect`（`open` 变化重置块）的 `if (open) { ... }` 体内，追加 `setFetchError(null)`：

```ts
  useEffect(() => {
    if (open) {
      setName("")
      setShowAdvanced(false)
      setCustomBranch("")
      setBranchSearch("")
      setFetchError(null)
      // Restore last selected baseRef from settings store
      setBaseRef(lastBaseRef)
    }
  }, [open, lastBaseRef])
```

- [ ] **Step 4: 改造刷新按钮 UI**

找到当前刷新按钮（"基于分支 / ref" Label 下方的 `<button>`），整体替换为：

```tsx
                  <button
                    type="button"
                    onClick={handleRefresh}
                    disabled={fetchMutation.isPending || repoInfoFetching}
                    className="relative text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
                    title={fetchError ? `远端刷新失败：${fetchError}，显示为本地缓存` : "刷新分支列表"}
                  >
                    <RefreshCw
                      className={cn(
                        "w-3 h-3",
                        (fetchMutation.isPending || repoInfoFetching) && "animate-spin"
                      )}
                    />
                    {fetchError && (
                      <span className="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 rounded-full bg-red-500" />
                    )}
                  </button>
```

- [ ] **Step 5: 类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误

- [ ] **Step 6: 提交**

```bash
git add src/components/worktree/CreateWorktreeDialog.tsx && git commit -m "feat(worktree): 刷新按钮接入 git fetch 与失败提示"
```

---

## Task 5: 全量验证

- [ ] **Step 1: 跑全部 Rust 测试**

Run: `cd src-tauri && cargo test`
Expected: 全部通过，含 3 个新测试

- [ ] **Step 2: 前端类型检查**

Run: `npx tsc --noEmit`
Expected: 无错误

- [ ] **Step 3: 手动验证（可选，由用户在 `npm run tauri dev` 下执行）**

- 断网点刷新 → 转圈 → fetch 失败后出现红点 + tooltip"远端刷新失败：...，显示为本地缓存"，分支列表仍展示本地缓存。
- 联网点刷新 → 能看到远端新建分支。
- 对话框首次打开不触发 fetch（无网络延迟）。
- 日志文件 `%USERPROFILE%\.claude-fleet\logs\claude-fleet-YYYY-MM-DD.log` 应见 `[fetch_repo_remotes_cmd]` 与 `[fetch_remotes]` 条目。
