# Windows Terminal 跳转方案设计（AttachConsole + GetConsoleWindow）

- 日期：2026-07-20
- 目标：让 Claude Fleet 支持"跳转到 Windows Terminal 指定 tab"。
- 现状：`docs/KNOWN_ISSUES.md` 第 1 条记录 WT 不受支持。根因——WT 所有 tab 共用同一个 `WindowsTerminal.exe` 进程，现有 `find_window_by_pid_chain` 从 claude PID 逐层向上找持有窗口的父进程，最终命中的是 WT 主进程；而 WT 主进程持有多个窗口（主窗口 + 各 tab 的不可见 pseudo-console 宿主窗口），`EnumWindows` 只能拿到"第一个"，无法定位到 claude 实际所在的那个 tab。

## 1. 调研结论（来源：microsoft/terminal#12570 / #2988 / #12515 / #12526 / #12799 / #12899 / #12900）

1. **WT 为每个 tab/pane 维护一个"不可见的 pseudo-console 宿主窗口"**，使其对 `GetConsoleWindow()` 返回一个**唯一** HWND（per console session / per tab）。这是为兼容 `msys`/`cygwin` 用 console HWND 作为 tty 标识的历史用法而设计的——每个 tab 的 pseudo HWND 各不相同。
2. 因此，对一个跑在 WT 某个 tab 里的 claude 进程调用 `AttachConsole(claude_pid)` 后，`GetConsoleWindow()` 返回的正是**该 tab 对应的 pseudo HWND**——天然解决了"同一父进程多窗口无法区分"的问题。
3. **WT v1.14 起**，对 pseudo HWND 的 show/hide/foreground/z-order 操作会**传播到真实 WT 主窗口并切换到正确的 tab**（PR #12515 show/hide、#12526 owner 传递、#12799 子窗口 z-order、#12899 FG 权限校验、#12900 focus 事件）。所以对 pseudo HWND 调 `SetForegroundWindow` 即可把 WT 拉到前台并切到正确 tab。
4. 副作用收益：该方案对 cmd / powershell / powershell7 / wezterm 同样有效（它们都给 claude 分配了真实 console：经典 conhost 或 ConPTY），且**比现有"父链向上找"更快**（几次内核调用 vs 多次 EnumWindows + ToolHelp 快照）。
5. 失败场景：**Git Bash（mintty + winpty/pty）** 给 claude 的是伪终端而非 Windows console，`AttachConsole` 会失败 → 回退到现有父链逻辑（保持 KNOWN_ISSUES 第 5 条现状）。

## 2. 关键约束与性能

- `AttachConsole` / `FreeConsole` / `GetConsoleWindow` 操作**进程级** console 状态：一个进程同一时刻只能 attach 到一个 console。**不能跨线程并行**调用 attach 序列，否则相互覆盖。
- 因此 attach 序列必须用**全局 Mutex 串行化**。但该序列本身极快（微秒级，纯内核调用，无 IPC、无子进程），串行不构成瓶颈。
- 慢路径仍是父链查找（200–500ms，含 ToolHelp 快照 + 多次 EnumWindows）。串行化只覆盖 attach 部分；父链部分仍可像现有实现那样**跨线程并行**。
- 结论：`populate_window_cache_parallel` 仍可 per-pid 起线程；每个线程内部先（持锁）跑 attach 快路径，命中即返回；未命中再在**锁外**跑父链慢路径。多线程并行性不受影响。

## 3. 后端 API 设计（`utils/window_manager.rs`）

### 3.1 新增全局串行锁

```rust
#[cfg(target_os = "windows")]
static CONSOLE_ATTACH_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
```

### 3.2 新增 `find_window_by_console_attach`

```rust
/// 通过 AttachConsole + GetConsoleWindow 定位进程所在 console 的窗口 HWND。
/// 适用于 ConPTY 宿主（Windows Terminal、WezTerm）与经典 conhost（cmd/powershell）。
/// 对 WT：返回对应 tab 的不可见 pseudo-console 宿主窗口（唯一 per-tab）。
/// 对 Git Bash（mintty，无真实 console）：AttachConsole 失败，返回 None。
///
/// 注意：AttachConsole/FreeConsole/GetConsoleWindow 操作进程级 console 状态，
/// 必须串行调用，故持全局 CONSOLE_ATTACH_MUTEX。
#[cfg(target_os = "windows")]
pub fn find_window_by_console_attach(pid: u32) -> Option<HWND> {
    let _guard = CONSOLE_ATTACH_MUTEX.lock().unwrap();
    // GUI 进程通常无 console；先释放可能残留的 attach，避免 AttachConsole 报错
    unsafe { let _ = FreeConsole(); }
    let attached = unsafe { AttachConsole(pid) };
    if !attached.is_ok() {  // BOOL → 失败记日志
        debug!("[find_window_by_console_attach] AttachConsole 失败 pid={}", pid);
        return None;
    }
    let hwnd = unsafe { GetConsoleWindow() };
    // 立即释放，避免影响后续调用 / 其它线程
    unsafe { let _ = FreeConsole(); }
    if hwnd.is_invalid() /* 0 或 null */ { return None; }
    // 校验是真实窗口
    if unsafe { !IsWindow(hwnd).as_bool() } { return None; }
    Some(hwnd)
}
```

要点：
- `_guard` 在函数返回时自动解锁，保证异常路径也释放。
- `FreeConsole` 在 attach 前调用一次，保证自身无残留 attach（Tauri 为 GUI subsystem，正常情况无 console，FreeConsole 返回 false 不影响）。
- `GetConsoleWindow` 返回的 HWND 在 `FreeConsole` 之后**仍然有效**——它只是窗口句柄，不依赖 attach 状态；窗口随 console session 存活。

### 3.3 resolve 优先级调整

新增统一解析入口（缓存 → attach 快路径 → 父链慢路径）：

```rust
#[cfg(target_os = "windows")]
pub fn resolve_window_for_pid(pid: u32) -> Option<HWND> {
    // 1. attach 快路径（WT/cmd/ps/wezterm，微秒级，串行）
    if let Some(hwnd) = find_window_by_console_attach(pid) {
        info!("[resolve_window_for_pid] attach 路径命中 pid={}", pid);
        return Some(hwnd);
    }
    // 2. 父链慢路径（git bash 等无 console 场景；并行由调用方负责）
    if let Some(hwnd) = find_window_by_pid_chain(pid) {
        info!("[resolve_window_for_pid] 父链路径命中 pid={}", pid);
        return Some(hwnd);
    }
    None
}
```

`resolve_and_cache_window` 改为调用 `resolve_window_for_pid`（保留原签名，缓存写入逻辑不变，owner_pid 仍由 `GetWindowThreadProcessId` 读取）。对 WT，owner_pid 是 WT 进程（多 tab 共享），但因缓存 key 是 claude pid、每个 claude pid 独立指向各自的 pseudo HWND，不影响正确性。

### 3.4 缓存验证的 WT 适配

现有 `get_cached_window` 用 `IsWindow + GetWindowThreadProcessId == owner_pid` 校验。对 WT：
- pseudo HWND 在 tab 关闭时被销毁 → `IsWindow=false` 能正确失效。✓
- owner_pid 为 WT 主进程，多 tab 共享 → 无法检测"claude 进程死亡但 tab 仍开着（用户在 tab 里又敲了别的命令）"导致的 stale。该场景靠 `invalidate_window_cache(pid)` 在 session 移除时清理（现有逻辑已覆盖，WT session 同样走该路径）。PID 复用风险与 cmd/ps 一致，不额外处理。

### 3.5 `populate_window_cache_parallel` 微调

保持 per-pid 起线程；线程内调用 `resolve_and_cache_window`（已走 attach 优先）。attach 序列自带全局锁串行，父链部分在锁外并行。**无需改动并行结构**，只需让 `resolve_and_cache_window` 内部优先 attach。

### 3.6 激活 WT pseudo 窗口的特殊处理

pseudo HWND 不可见。直接复用 `activate_window`：`IsIconic/IsZoomed` 对不可见窗口返回 false，`ShowWindow(SW_SHOW)` 为 no-op，退化为 `SetForegroundWindow + BringWindowToTop + Alt trick`，由 WT 传播到真实窗口+正确 tab。

为覆盖"WT 主窗口被最小化"场景，在激活 pseudo HWND 前，先用 `GetAncestor(hwnd, GA_ROOT)` 取真实顶层窗口；若 `IsIconic(root)`，先 `ShowWindow(root, SW_RESTORE)`。新增辅助：

```rust
#[cfg(target_os = "windows")]
pub fn activate_console_window(hwnd: HWND) -> Result<(), String> {
    unsafe {
        let root = GetAncestor(hwnd, GA_ROOT);
        if !root.is_invalid() && IsIconic(root).as_bool() {
            let _ = ShowWindow(root, SW_RESTORE);
        }
    }
    activate_window(hwnd)  // 复用 Alt trick + SetForegroundWindow + BringWindowToTop
}
```

`jump_to_terminal_by_pid` / `smart_jump_to_terminal` 命中 attach 路径（WT）时改用 `activate_console_window`；父链路径维持 `activate_window`。区分方式：attach 路径返回的 HWND 不可见——可在 `resolve_window_for_pid` 返回时附带一个 `is_pseudo` 标记，或简单地：始终对 attach 路径结果调用 `activate_console_window`（对 cmd/ps 的 conhost 窗口也安全：GetAncestor 取到的 root 就是它自己，IsIconic 正常工作）。

为简化，决定：**所有 `resolve_window_for_pid`/`resolve_and_cache_window` 命中的结果统一走 `activate_console_window`**（它对普通窗口也正确，只是多了根窗口最小化恢复，是纯增强）。

## 4. WT 终端类型接入（launch）

### 4.1 后端 `utils/launch/mod.rs` `build_spawn_plan` 新增分支

```rust
"windows-terminal" => Ok(SpawnPlan {
    command: "wt.exe".to_string(),
    args: {
        let mut v = vec![
            "-d".to_string(),
            request.working_directory.clone(),
            "cmd".to_string(),
            "/K".to_string(),
        ];
        v.push(command_line(&process_argv));
        v
    },
    current_dir: None,
    creation_flags: Some(DETACHED_PROCESS),
}),
```

- 用 `cmd /K` 包裹 claude，与 `cmd` 终端类型行为一致（claude 退出后 tab 保留，便于看结束信息）。
- `-d <cwd>` 让 cmd 在目标目录启动；WT 的 `-d` 等价于 `--cwd`（starting directory）。
- `wt.exe` 依赖 `%LOCALAPPDATA%\Microsoft\WindowsApps` 在 PATH（Windows 默认开启）。未找到时报错"启动终端失败"，日志可见。
- `DETACHED_PROCESS` 与 wezterm 一致（GUI 终端，解耦父子进程）。
- CommandWrapper（ccglass）**不跳过**：WT 走 `cmd /K "<wrapper argv quoted>"`，与 cmd 一致。

### 4.2 前端

- `src/types/settings.ts`：`TerminalType` 增加 `'windows-terminal'`。
- `src/components/dialogs/SettingsDialog.tsx`：`TERMINAL_OPTIONS` 增加 `{ value: 'windows-terminal', label: 'Windows Terminal' }`。
- `src/stores/settingsStore.ts`：`isTerminalType` 增加 `'windows-terminal'` 判定。
- 迁移兼容：用户旧设置 `terminalType: 'wezterm'` 不受影响；WT 为新增可选项。

## 5. 测试策略

### 5.1 Rust 单元测试（`window_manager.rs` / `launch/mod.rs`）

- `build_spawn_plan` 对 `windows-terminal` 生成正确 args（`-d cwd cmd /K "<cmdline>"`、creation_flags=DETACHED_PROCESS）。
- `windows-terminal` 不跳过 wrapper（与 wezterm 行为相反的回归测试）。
- `find_window_by_console_attach` 在非 Windows 平台编译（`#[cfg(not)]` 占位返回 None）。
- 缓存读写不变（既有测试不动）。

attach 真实 Win32 行为**不写单测**（依赖真实进程/控制台，CI 上不可复现），改由人工验收（编译 exe 后在真实 WT 里跑）。

### 5.2 前端

- `npx tsc --noEmit` 通过（新增 union 成员被覆盖）。

### 5.3 人工验收（编译 exe 后由用户完成）

1. 设置终端类型为 Windows Terminal，新建一个 claude session。
2. 在 WT 内开多个 tab，确认"跳转"始终切到 claude 所在 tab 并把 WT 拉到前台。
3. WT 最小化时跳转能恢复并切到正确 tab。
4. cmd/powershell/wezterm 跳转不回归。

## 6. 文档更新

- `docs/KNOWN_ISSUES.md` 第 1 条标记为已解决（v0.8.x），保留 Git Bash 第 5 条不变。
- `CLAUDE.md` 终端集成表增加 windows-terminal 行（`wt.exe -d <cwd> cmd /K <cmd>`，`DETACHED_PROCESS`）。

## 7. 变更清单

| 文件 | 变更 |
|---|---|
| `src-tauri/src/utils/window_manager.rs` | 新增 `CONSOLE_ATTACH_MUTEX`、`get_owner_process_name`、`is_windows_terminal_owner`、`find_window_by_console_attach`、`resolve_window_for_pid`、`activate_console_window`、`is_cached_console_window`；`WindowCacheEntry` 加 `is_console_window`；`resolve_and_cache_window` 走新入口；非 Windows 占位 |
| `src-tauri/src/commands/terminal.rs` | jump 命令慢路径改走 `resolve_window_for_pid`（不再直接调 `find_window_by_pid_chain`）；激活按 `is_console_window` 选 `activate_console_window` / `activate_window` |
| `src-tauri/src/utils/launch/mod.rs` | `build_spawn_plan` 新增 `windows-terminal` 分支 + 测试 |
| `src-tauri/src/utils/running_sessions.rs` | `refresh_session_names` 对 pseudo console session 跳过父链标题查询、用文件夹名 |
| `src/types/settings.ts` | `TerminalType` 增加 `'windows-terminal'` |
| `src/components/dialogs/SettingsDialog.tsx` | `TERMINAL_OPTIONS` 增加 Windows Terminal |
| `src/stores/settingsStore.ts` | `isTerminalType` 增加 `windows-terminal` |
| `docs/KNOWN_ISSUES.md` | 第 1 条标记已解决 |
| `CLAUDE.md` | 终端集成表补 windows-terminal 行 |

## 8. subagent 审核后修订

1. **G1 关键缺口**：原方案未覆盖 `commands/terminal.rs` 慢路径仍直接调 `find_window_by_pid_chain`（绕过 attach）。修订：慢路径改走 `resolve_window_for_pid`，否则首次跳转命中的是 WT 主窗口而非 pseudo HWND，切 tab 失败。
2. **G2 wezterm 回归**：attach 对 wezterm（ConPTY 由 OpenConsole 宿主）会返回不可见且不响应 foreground 的窗口，导致 wezterm 跳转从"切正确 pane"退化为"什么都不切"。修订：attach 命中后用 `GetWindowThreadProcessId` 取 owner_pid，再用 `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)+QueryFullProcessImageNameW` 取 owner 进程名，**仅当 owner 进程名含 `WindowsTerminal`（大小写不敏感）时**才采用 attach 路径；否则丢弃、回退父链。cmd/ps/wezterm/git-bash 行为与现状完全一致（零回归）。提权场景 OpenProcess 失败 → 回退父链，可接受。
3. **F 标题串扰**：WT session 经 `get_window_title_by_pid_chain` 父链会拿到 WT 主窗口标题（反映当前活动 tab，且随切换抖动），导致同一 WT 下所有 session 共享一个抖动标题。修订：`WindowCacheEntry` 增加 `is_console_window: bool`（attach 且确认为 WT 时置 true）；`refresh_session_names` 对 `is_console_window=true` 的 session 直接用文件夹名、**跳过父链标题查询**（不 push 进 `uncached_pids`）。
4. **D 激活专版**：不复用 `activate_window`（其 `ShowWindow(hwnd, SW_SHOW)` 会打到 pseudo HWND，WT <1.14 可能产生 0-size 残影；≥1.14 会重复触发 show）。修订：写 `activate_console_window`——用 `GetAncestor(hwnd, GA_ROOTOWNER)` 取真实 WT 顶层窗口；`IsIconic(root)` 则 `ShowWindow(root, SW_RESTORE)`，否则若 `!IsWindowVisible(root)` 则 `ShowWindow(root, SW_SHOW)`；再 Alt trick + `SetForegroundWindow(pseudo)`（WT 路由到正确 tab，关键不可改对 root 调）+ `BringWindowToTop(pseudo)`；**不对 pseudo 调 SW_SHOW**。
5. **B 线程安全细化**：`CONSOLE_ATTACH_MUTEX.lock().unwrap_or_else(|e| e.into_inner())` 防 mutex 中毒连锁；attach 序列内 `info!/debug!` 移出持锁段（先取 HWND 出锁再日志），避免 tracing I/O 持锁；在 mutex 定义处加注释"持锁时禁止获取任何其它 mutex"防 AB-BA。
6. **A 边界**：`find_window_by_console_attach` 入口加 `if pid == GetCurrentProcessId() { return None }` 短路；attach 前置 `FreeConsole` 仅清理自身残留（Tauri GUI subsystem 通常无 console，FreeConsole 返回 false 不影响）。
7. **E 文档化**：`wt.exe` 依赖 `%LOCALAPPDATA%\Microsoft\WindowsApps` App Execution Alias 在 PATH（Windows 默认开启），未启用时启动失败、日志可见；`cmd /K "<quoted>"` 经 Rust `Command` 二次引号包装，含 `&` 等特殊字符的目录名可能出错（与既有 `cmd` 分支同源，列为已知限制）。
8. **非 Windows 占位**：`find_window_by_console_attach` / `resolve_window_for_pid` / `activate_console_window` / `is_cached_console_window` / `get_owner_process_name` / `is_windows_terminal_owner` 均加 `#[cfg(not(target_os="windows"))]` 占位实现。
9. **WT 版本前置**：要求 Windows Terminal ≥ 1.14（2022 年起所有正式版均满足）；更低版本切 tab 可能失败，回退父链（仅激活主窗口，不切 tab）。

