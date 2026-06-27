# Worktree 创建对话框刷新按钮接入 git fetch 设计

日期：2026-06-27
状态：已批准（待实现）

## 背景

新建 Worktree 对话框（`CreateWorktreeDialog`）中"基于分支 / ref"旁有一个刷新按钮。当前点击它仅重新读取本地 git 仓库状态（本地分支 + 远程跟踪分支 `refs/remotes/*` + remotes 列表 + 默认分支），调用链：

```
RefreshCw onClick → refetchRepoInfo()
  → useRepoInfoQuery (TanStack Query, staleTime 5 分钟)
  → worktreesApi.getRepoInfo(repoPath)
  → invoke("get_repo_info_cmd")
  → get_remotes / get_local_branches / get_remote_branches / get_default_branch
```

这些底层函数全是只读 git 命令（`git remote -v`、`git branch --list`、`git branch -r`、`git symbolic-ref` 等），**不执行 `git fetch`**。因此"远程分支"列表实为本地缓存的远程跟踪引用快照，远端新建分支若未 fetch 过则看不到。

## 目标

让刷新按钮在重新读取分支列表前，先执行 `git fetch` 拉取远端最新引用，使用户能看到远端最新分支。对话框首次打开仍走本地缓存（快、无网络依赖），仅用户主动点刷新时才联网。

## 设计决策（已与用户确认）

1. **触发时机**：仅刷新按钮触发 fetch；对话框打开不自动 fetch。
2. **Fetch 范围**：`git fetch --all --prune`，拉取所有已配置 remote（origin、upstream 等）并清理已删除的远程分支。
3. **错误处理**：降级——fetch 失败（无网络/鉴权失败/超时）时仍展示本地缓存分支，并在刷新图标处给出提示。
4. **超时与进度**：fetch 设 30 秒超时（超时 kill 进程）；fetching 期间复用现有 `RefreshCw` 转圈动画。

## 方案选型

采用**方案 A：新增独立命令 `fetch_repo_remotes_cmd`**。

- fetch（副作用变更）与 repoInfo（只读查询）职责分离，符合 TanStack Query 习惯。
- fetch 失败的"降级"语义天然由"fetch 命令返回结构化结果 + 前端总是 refetchRepoInfo"表达，不污染 `RepoInfo` 类型。
- 备选方案 B（给 `get_repo_info_cmd` 加 `fetch: bool`）与方案 C（合并命令 `refresh_repo_info_cmd`）均把读/写混入同一入口或与现有命令重叠，未采用。

## 详细设计

### 1. 后端工具函数 `fetch_remotes`

文件：`src-tauri/src/utils/git/mod.rs`

新增独立于 `execute_git` 的带超时 fetch（`execute_git` 用 `.output()` 阻塞，无法超时）。

签名：

```rust
pub fn fetch_remotes(repo_path: &Path, timeout_secs: u64) -> Result<(), String>
```

实现要点：

- `crate::utils::process::command("git").arg("-C").arg(repo_path).args(["fetch", "--all", "--prune"])`
- `stdout(Stdio::null())`、`stderr(Stdio::piped())`、`spawn()`
- 起一个线程排空 stderr 到 `String`（避免管道写满导致子进程阻塞死锁）
- 主循环：`child.try_wait()`，已退出则取 status；未到 deadline 则 `sleep(200ms)` 继续轮询；到达 deadline 则 `child.kill()` + `child.wait()` 收尸，join stderr 线程，返回 `Err("git fetch 超时（{timeout_secs}s）")`
- 成功（`status.success()`）返回 `Ok(())`
- 失败：stderr trim 非空则 `Err("git fetch 失败: {stderr}")`，否则 `Err("git fetch 失败（exit {code}）")`
- 日志：入口 `info!("[fetch_remotes] 开始: repo={}, timeout={}s", ...)`；成功 `info!("[fetch_remotes] 完成")`；失败/超时 `warn!("[fetch_remotes] {}", msg)`
- 导入：`use std::io::Read;` `use std::process::Stdio;` `use std::time::{Duration, Instant};`

### 2. 后端命令与类型

文件：`src-tauri/src/commands/worktree.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResult {
    pub success: bool,
    pub message: Option<String>,
}

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

关键点：fetch 失败/超时时**仍返回 `Ok(FetchResult { success: false, message: Some(e) })`**——降级语义，fetch 失败不是致命错误，前端据此显示提示但继续走本地缓存。仅当命令本身无法执行（路径异常等）才返回 `Err`。

注册：

- `src-tauri/src/commands/mod.rs`：导出 `fetch_repo_remotes_cmd`（及 `FetchResult` 若前端/其它模块需要，此处仅导出命令）。
- `src-tauri/src/lib.rs`：`use ...fetch_repo_remotes_cmd` 并加入 `invoke_handler` 的 `tauri::generate_handler!` 列表（紧跟 `get_repo_info_cmd` 之后）。

### 3. 前端类型与 API

`src/types/worktree.ts`：

```ts
export interface FetchResult {
  success: boolean
  message: string | null
}
```

`src/types/index.ts`：追加导出 `FetchResult`（与 `RepoInfo` 等同文件来源一致）。

`src/lib/api/worktrees.ts`：

```ts
async fetchRepoRemotes(repoPath: string): Promise<FetchResult> {
  return await invoke("fetch_repo_remotes_cmd", { repoPath })
}
```

`src/lib/query/worktreeMutations.ts`：

```ts
export const useFetchRepoRemotesMutation = () => {
  return useMutation<FetchResult, Error, string>({
    mutationFn: (repoPath: string) => worktreesApi.fetchRepoRemotes(repoPath),
  })
}
```

沿用 `queryClient` 全局 `retry=false` 配置，fetch 失败不重试。

### 4. 前端交互

文件：`src/components/worktree/CreateWorktreeDialog.tsx`

新增状态与 mutation：

```ts
const [fetchError, setFetchError] = useState<string | null>(null)
const fetchMutation = useFetchRepoRemotesMutation()
```

刷新按钮 `onClick` 改为 `handleRefresh`：

```ts
const handleRefresh = async () => {
  setFetchError(null)
  try {
    const res = await fetchMutation.mutateAsync(repoPath)
    await refetchRepoInfo() // 无论 fetch 成功失败都刷新本地分支视图
    if (!res.success && res.message) {
      setFetchError(res.message)
    }
  } catch {
    // invoke 级传输错误，由 mutation onError 静默吞掉；分支列表不刷新
  }
}
```

UI 反馈：

- 转圈条件：`fetchMutation.isPending || repoInfoFetching`，复用现有 `<RefreshCw className={cn("w-3 h-3", spinning && "animate-spin")} />`。
- 失败提示：刷新按钮容器 `title`——成功时"刷新分支列表"，失败时"远端刷新失败：{fetchError}，显示为本地缓存"。
- 红点：`fetchError` 非空时在图标右上角显示一个小红点（绝对定位 `bg-red-500` 圆点）。
- 清空：下次点击刷新时 `setFetchError(null)`；对话框关闭时在现有 `useEffect`（`open` 变化的重置块）中一并 `setFetchError(null)`。

对话框首次打开**不**触发 fetch，仅 `repoInfoLoading` 走本地缓存——符合"仅刷新按钮触发"。

### 5. 测试

文件：`src-tauri/src/commands/worktree.rs` 的 `#[cfg(test)]` 模块，匹配现有基线（serde 往返 + 纯逻辑/本地 IO），新增三项：

1. `fetch_result_camel_case_roundtrip`——构造 `FetchResult` 的 success 与 failure 两种变体，断言 JSON 含 `success`/`message`、无下划线，且能往返解析。

2. `fetch_remotes_returns_err_on_non_git_dir`——用 `std::env::temp_dir()` + 原子计数器（复用现有 `COUNTER`，见 `git/mod.rs:376` 测试）生成唯一临时空目录，调用 `fetch_remotes(&dir, 30)`，断言返回 `Err` 且消息非空。快速、确定性、无网络，覆盖 spawn+wait+stderr 错误路径。测试结束清理目录。

3. `fetch_remotes_success_against_local_bare`——用临时目录构建：
   - bare 仓库 `remote.git`（`git init --bare`）
   - 工作仓库 `work`（`git init`，配置 `user.email`/`user.name`，`git remote add origin <remote 绝对路径>`）
   - 在 `work` 中造一个提交并 `git push origin main`（或直接在 remote 侧造引用）
   - 删除 `work` 的远程跟踪引用后调用 `fetch_remotes(&work, 30)`，断言 `Ok(())` 且 `get_remote_branches(&work)` 含 `origin/main`
   - 无网络依赖（file:// 远端），覆盖 happy path 与 `--prune` 行为；测试结束清理两个目录。

   所有 git 调用通过 `crate::utils::process::command("git")`（避免 Windows 弹窗）。

> 无需引入 `tempfile` 依赖：复用 `std::env::temp_dir()` + 现有 `COUNTER` 即可生成唯一路径，手动 `fs::remove_dir_all` 清理。

### 6. 改动文件清单

| 文件 | 改动 |
|---|---|
| `src-tauri/src/utils/git/mod.rs` | 新增 `fetch_remotes` 函数 + 相关导入 |
| `src-tauri/src/commands/worktree.rs` | 新增 `FetchResult` 结构、`fetch_repo_remotes_cmd` 命令、3 个测试 |
| `src-tauri/src/commands/mod.rs` | 导出 `fetch_repo_remotes_cmd` |
| `src-tauri/src/lib.rs` | 注册 `fetch_repo_remotes_cmd` |
| `src/types/worktree.ts` | 新增 `FetchResult` |
| `src/types/index.ts` | 导出 `FetchResult` |
| `src/lib/api/worktrees.ts` | 新增 `fetchRepoRemotes` |
| `src/lib/query/worktreeMutations.ts` | 新增 `useFetchRepoRemotesMutation` |
| `src/components/worktree/CreateWorktreeDialog.tsx` | 刷新按钮 handler + 失败提示 UI + 状态 |

## 成功标准

- `cd src-tauri && cargo test` 全绿（含 3 个新测试）。
- `npx tsc --noEmit` 无类型错误。
- 手动验证：
  - 断网点刷新 → 转圈 → fetch 失败后出现红点 + tooltip，分支列表仍展示本地缓存。
  - 联网点刷新 → 能看到远端新建分支。
  - 对话框首次打开不触发 fetch（无网络延迟）。

## 非目标

- 不在对话框打开时自动 fetch。
- 不为 fetch 添加详细进度条（仅转圈 + 超时）。
- 不引入可配置的超时时间（固定 30s）。
- 不改造 `get_repo_info_cmd` 或其它既有命令。
