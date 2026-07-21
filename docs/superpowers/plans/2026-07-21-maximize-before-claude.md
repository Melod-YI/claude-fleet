# 渲染前最大化（helper 子命令）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把"打开终端时最大化"重构为 helper 子命令前置最大化（在 claude 渲染前完成），覆盖 cmd/ps/ps7（含 WT 宿主与经典 conhost），wezterm 不支持最大化，并移除 Tauri 主进程内 AttachConsole 的旧机制。

**Architecture:** `claude-fleet.exe maximize-window` 子命令在 helper 进程内最大化当前/父终端窗口后 exit 0；`build_spawn_plan` 在 `maximize_window=true` 时把终端命令构造为 `"<helper> maximize-window && claude..."`（helper 先跑、终端再启动 claude → 最大化先于渲染）。helper 进程短命，AttachConsole 污染随其退出消亡，Tauri 主进程永不调用 AttachConsole，根除 os error 50 与点击陷阱。

**Tech Stack:** Rust + Tauri 2.0（windows crate 0.58 Win32 API），React+TS 前端。

设计文档：`docs/superpowers/specs/2026-07-21-maximize-before-claude-design.md`

---

## 文件结构

- **修改** `src-tauri/src/main.rs`：CLI 子命令分发（`argv[1] == "maximize-window"` 时调 helper 后退出）。
- **修改** `src-tauri/src/lib.rs`：新增 `pub fn maximize_current_process_window()` 桥接（供 main.rs 调用，跨平台守卫）。
- **修改** `src-tauri/src/utils/window_manager.rs`：新增 `maximize_current_process_window()` + 辅助函数；删除旧 `maximize_terminal_window` / `resolve_visible_target_from_pseudo`（保留 `visible_titled_root_owner`，新 helper 复用）。
- **修改** `src-tauri/src/utils/launch/mod.rs`：`build_spawn_plan` 在 `maximize_window=true` 时为 cmd/ps/ps7 构造 helper 前缀；wezterm warn 跳过；`launch_session` 删除后台线程最大化分支。
- **修改** `src-tauri/src/utils/process.rs`：`reset_std_handles` 还原为私有 `fn`（之前为 maximize 调用改的 `pub(crate)` 不再需要）。
- **修改** `src/components/dialogs/SettingsDialog.tsx`：wezterm 时 maximize 开关置灰 + 提示。

---

## Task 1: helper 子命令（console 路径）+ spike 验证

**Goal:** 落地 CLI 子命令分发 + helper 的"阶段1+2（AttachConsole 父 console → console 路径）"，release 构建后手动在 cmd+WT 下验证 WT 真最大化、无 error 50、无陷阱。**失败立即回报，不继续后续任务。**

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/utils/window_manager.rs`

- [ ] **Step 1: main.rs 加子命令分发**

替换 `src-tauri/src/main.rs` 全部内容为：

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 子命令分发：`claude-fleet.exe maximize-window` —— 启动终端时由终端命令前置调用，
    // 在本进程内最大化当前/父终端窗口后退出，Tauri 主进程不进入。
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1] == "maximize-window" {
        if let Err(e) = claude_fleet::maximize_current_process_window() {
            eprintln!("[maximize-window] 失败: {}", e);
        }
        // best-effort：无论成败都 exit 0，绝不阻塞 claude 启动
        std::process::exit(0);
    }
    claude_fleet::run()
}
```

- [ ] **Step 2: lib.rs 加 pub 桥接函数**

在 `src-tauri/src/lib.rs` 的 `pub fn run()` 函数**之前**插入：

```rust
/// CLI 子命令 `maximize-window` 的入口：在 helper 进程内最大化当前/父终端窗口。
/// 由 main.rs 在解析到 `maximize-window` 子命令时调用，不进入 Tauri。
pub fn maximize_current_process_window() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::utils::window_manager::maximize_current_process_window()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("仅支持 Windows 平台".to_string())
    }
}
```

- [ ] **Step 3: window_manager.rs 加 helper 函数（console 路径）**

在 `src-tauri/src/utils/window_manager.rs` 文件末尾的 `#[cfg(test)] mod tests` **之前**插入：

```rust
/// CLI 子命令 `maximize-window` 的实现：在**当前 helper 进程**内最大化所在终端窗口。
///
/// 供 launch 构造的终端命令前置调用（如 `cmd /K "claude-fleet.exe maximize-window && claude ..."`）：
/// helper 进程短命，AttachConsole 污染随进程退出消亡，Tauri 主进程永不调用 AttachConsole，
/// 杜绝 os error 50 与点击陷阱。
///
/// 流程：
/// 1. AttachConsole(ATTACH_PARENT_PROCESS) 挂到父进程（终端进程）的 console。
/// 2. GetConsoleWindow 取 pseudo/conhost 窗口 → 解析【可见+有标题】目标（WT 走 GetAncestor
///    GA_ROOTOWNER，conhost 直接用，兜底 find_window_by_pid(owner_pid)）→ ShowWindow(SW_MAXIMIZE)。
/// 3. 阶段2 拿不到可见目标（wezterm ConPTY 等）→ 沿父链找可见+有标题祖先窗口，
///    跳过 image 名含 claude-fleet 的祖先（防误最大化 app 自身）→ ShowWindow。（Task 2 实现）
/// 4. 始终返回 Ok（best-effort，绝不阻塞 claude）。
#[cfg(target_os = "windows")]
pub fn maximize_current_process_window() -> Result<(), String> {
    let pid = unsafe { GetCurrentProcessId() };
    info!("[maximize_current_process_window] 开始，pid={}", pid);

    // 阶段1+2：attach 父 console → console 路径
    if let Some(target) = resolve_console_target() {
        unsafe {
            let ok = ShowWindow(target, SW_MAXIMIZE).as_bool();
            info!(
                "[maximize_current_process_window] console 路径命中 target={} ShowWindow(SW_MAXIMIZE)={}",
                target.0 as usize, ok
            );
        }
        return Ok(());
    }

    // 阶段3 父链兜底由 Task 2 补充；当前直接跳过
    warn!(
        "[maximize_current_process_window] console 路径未命中可见窗口，跳过最大化（阶段3 未实现），pid={}",
        pid
    );
    Ok(())
}

/// AttachConsole(ATTACH_PARENT_PROCESS) + GetConsoleWindow 解析【可见+有标题】的终端窗口。
///
/// - WT pseudo（不可见）：GetAncestor(GA_ROOTOWNER) 取宿主 WT 主窗口（visible_titled_root_owner）。
/// - conhost / cmd 自持：hwnd 须可见+有标题才采用（防 pseudo 陷阱）。
/// - 兜底：find_window_by_pid(owner_pid) 枚举 owner 进程的可见+有标题窗口。
///
/// 失败/无可见目标返回 None（调用方走阶段3 父链兜底）。
#[cfg(target_os = "windows")]
fn resolve_console_target() -> Option<HWND> {
    unsafe {
        // 清理自身可能残留的 attach（helper 通常无 console，FreeConsole 无副作用）
        let _ = FreeConsole();
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            debug!("[resolve_console_target] AttachConsole(父) 失败（父无 console），走父链兜底");
            return None;
        }
        let hwnd = GetConsoleWindow();
        // 立即释放 attach，避免影响后续
        let _ = FreeConsole();

        if hwnd.0.is_null() {
            debug!("[resolve_console_target] GetConsoleWindow 返回空");
            return None;
        }
        // WT：GetAncestor 取可见+有标题的宿主主窗口
        if is_windows_terminal_window(hwnd) {
            if let Some(root) = visible_titled_root_owner(hwnd) {
                debug!("[resolve_console_target] WT GetAncestor 命中可见有标题 root");
                return Some(root);
            }
        }
        // conhost 自持：须可见+有标题（防 pseudo 陷阱）
        if is_visible_titled(hwnd) {
            debug!("[resolve_console_target] conhost 窗口可见有标题，直接采用");
            return Some(hwnd);
        }
        // 兜底：枚举 owner 进程的可见+有标题窗口
        let mut owner_pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut owner_pid));
        if owner_pid != 0 {
            if let Some(real) = find_window_by_pid(owner_pid) {
                debug!(
                    "[resolve_console_target] owner_pid={} 兜底枚举命中可见有标题窗口",
                    owner_pid
                );
                return Some(real);
            }
        }
        None
    }
}

/// HWND 是否可见且有非空标题（pseudo 窗口无标题或不可见会被过滤）。
#[cfg(target_os = "windows")]
fn is_visible_titled(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let mut buf: [u16; 256] = [0; 256];
        GetWindowTextW(hwnd, &mut buf) > 0
    }
}
```

注：`visible_titled_root_owner`、`is_windows_terminal_window`、`find_window_by_pid`、`get_process_image_basename`、`get_parent_pid`、`GetCurrentProcessId`、`AttachConsole`/`FreeConsole`/`ATTACH_PARENT_PROCESS`/`GetConsoleWindow`/`ShowWindow`/`SW_MAXIMIZE`/`GetAncestor`/`GA_ROOTOWNER`/`IsWindowVisible`/`GetWindowThreadProcessId`/`GetWindowTextW`/`HWND` 均已在本文件存在或由顶部 `use windows::{...}` 覆盖，无需新增 import。

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo check --message-format=short`
Expected: `Finished` 无错误。

- [ ] **Step 5: release 构建（spike 用）**

Run: `cd src-tauri && cargo build --release`
Expected: `Finished` 产出 `target/release/claude-fleet.exe`。

- [ ] **Step 6: 手动 spike 验证（关键门槛）**

在 Windows Terminal 作为默认终端的环境下：

1. 打开一个 cmd（会被 WT 宿主），窗口保持非最大化。
2. 运行：`C:\workspace\claude-fleet-sp\src-tauri\target\release\claude-fleet.exe maximize-window`
3. 观察：该 WT 窗口应**立即最大化**。
4. 再手动启动几次 Claude Fleet 正常启动 session（开关 maximize），确认**无 os error 50、无点击陷阱**。

Expected: WT 真最大化；反复启动无 error 50/陷阱。

**若失败**：回报 `[maximize_current_process_window]` / `[resolve_console_target]` 的 stderr 输出与现象，**不要继续 Task 2+**。

- [ ] **Step 7: 提交**

```bash
cd C:/workspace/claude-fleet-sp
git add src-tauri/src/main.rs src-tauri/src/lib.rs src-tauri/src/utils/window_manager.rs
git commit -m "feat(maximize): helper maximize-window 子命令（console 路径）"
```

---

## Task 2: 父链兜底（阶段3）+ should_skip_ancestor 单测

**Goal:** 补全 helper 阶段3（console 路径未命中时沿父链找可见+有标题祖先窗口，跳过 app 自身），并加纯决策单测。

**Files:**
- Modify: `src-tauri/src/utils/window_manager.rs`

- [ ] **Step 1: 写 should_skip_ancestor 失败测试**

在 `src-tauri/src/utils/window_manager.rs` 末尾 `#[cfg(test)] mod tests` 内追加（与既有 `should_use_console_window_rule` 同级）：

```rust
    #[test]
    fn should_skip_ancestor_detects_app_self() {
        // image 名含 claude-fleet 即跳过（防误最大化 app 自身窗口）
        assert!(should_skip_ancestor("claude-fleet.exe"));
        assert!(should_skip_ancestor("Claude-Fleet.exe")); // 大小写不敏感
        // 终端进程不跳过
        assert!(!should_skip_ancestor("WindowsTerminal.exe"));
        assert!(!should_skip_ancestor("cmd.exe"));
        assert!(!should_skip_ancestor("powershell.exe"));
        assert!(!should_skip_ancestor("pwsh.exe"));
        assert!(!should_skip_ancestor("conhost.exe"));
        assert!(!should_skip_ancestor("wezterm.exe"));
        assert!(!should_skip_ancestor("OpenConsole.exe"));
    }
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test --lib should_skip_ancestor -- --nocapture`
Expected: 编译失败 `cannot find function should_skip_ancestor`（尚未定义）。

- [ ] **Step 3: 实现 should_skip_ancestor + 阶段3 函数**

在 `src-tauri/src/utils/window_manager.rs` 的 `is_visible_titled` 函数**之后**插入：

```rust
/// 父链兜底时是否跳过该祖先（image 名含 claude-fleet 即跳过，防误最大化 app 自身窗口）。
fn should_skip_ancestor(image_name: &str) -> bool {
    image_name.to_lowercase().contains("claude-fleet")
}

/// 沿父进程链向上查找第一个【可见+有标题】的窗口，跳过 image 名含 claude-fleet 的祖先。
/// 供 console 路径未命中时兜底（wezterm ConPTY 等）。
#[cfg(target_os = "windows")]
fn find_visible_titled_ancestor_excluding_app(start_pid: u32) -> Option<HWND> {
    let mut current_pid = start_pid;
    let mut depth = 0;
    const MAX_DEPTH: u32 = 10;
    while depth < MAX_DEPTH {
        let parent_pid = match get_parent_pid(current_pid) {
            Some(p) => p,
            None => break,
        };
        if parent_pid == 0 || parent_pid == current_pid {
            break;
        }
        // 跳过 app 自身（防误最大化 Claude Fleet 主窗口）
        if let Some(name) = get_process_image_basename(parent_pid) {
            if should_skip_ancestor(&name) {
                debug!(
                    "[find_visible_titled_ancestor_excluding_app] 跳过祖先 pid={} name={}",
                    parent_pid, name
                );
                current_pid = parent_pid;
                depth += 1;
                continue;
            }
        }
        if let Some(hwnd) = find_window_by_pid(parent_pid) {
            return Some(hwnd);
        }
        current_pid = parent_pid;
        depth += 1;
    }
    None
}
```

- [ ] **Step 4: 把阶段3 接入 maximize_current_process_window**

在 `src-tauri/src/utils/window_manager.rs` 的 `maximize_current_process_window` 中，把 Task 1 Step 3 写的"阶段3 父链兜底由 Task 2 补充；当前直接跳过"那段 warn 块替换为：

```rust
    // 阶段3：console 路径未命中 → 父链兜底（跳过 app 自身）
    if let Some(target) = find_visible_titled_ancestor_excluding_app(pid) {
        unsafe {
            let ok = ShowWindow(target, SW_MAXIMIZE).as_bool();
            info!(
                "[maximize_current_process_window] 父链兜底命中 target={} ShowWindow(SW_MAXIMIZE)={}",
                target.0 as usize, ok
            );
        }
        return Ok(());
    }

    warn!(
        "[maximize_current_process_window] 未找到可见终端窗口，跳过最大化，pid={}",
        pid
    );
    Ok(())
```

（即把原来的 `warn!(...)` 块整体替换为上面"阶段3 + 最终 warn"。）

- [ ] **Step 5: 运行测试确认通过**

Run: `cd src-tauri && cargo test --lib should_skip_ancestor -- --nocapture`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 6: 编译验证**

Run: `cd src-tauri && cargo check --message-format=short`
Expected: `Finished` 无错误。

- [ ] **Step 7: 提交**

```bash
cd C:/workspace/claude-fleet-sp
git add src-tauri/src/utils/window_manager.rs
git commit -m "feat(maximize): helper 阶段3 父链兜底 + should_skip_ancestor 单测"
```

---

## Task 3: build_spawn_plan helper 前缀 + wezterm 跳过

**Goal:** `build_spawn_plan` 在 `maximize_window=true` 时为 cmd/ps/ps7 构造 `helper maximize-window &&/; claude...` 命令；wezterm 不加前缀并 warn。

**Files:**
- Modify: `src-tauri/src/utils/launch/mod.rs`

- [ ] **Step 1: 加 current_exe 引号包装辅助函数**

在 `src-tauri/src/utils/launch/mod.rs` 的 `fn command_line` 函数**之前**插入：

```rust
/// 取当前 exe 路径并用双引号包装，供 helper 子命令调用。current_exe 失败返回 None。
fn current_exe_quoted() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\"", exe.display()))
}
```

- [ ] **Step 2: 改写 build_spawn_plan**

替换 `src-tauri/src/utils/launch/mod.rs` 中整个 `pub fn build_spawn_plan` 函数（原 118-166 行）为：

```rust
pub fn build_spawn_plan(request: &LaunchRequest) -> Result<SpawnPlan, String> {
    let process_argv = build_process_argv(request);
    if process_argv.is_empty() {
        return Err("启动命令为空".to_string());
    }
    let claude_cmdline = command_line(&process_argv);
    let maximize = request.settings.maximize_window;
    let terminal = request.settings.terminal_id.as_str();

    // wezterm 不支持最大化：即使 maximize=true 也不前缀 helper，warn 跳过
    if maximize && terminal == "wezterm" {
        tracing::warn!("[build_spawn_plan] wezterm 不支持最大化，已跳过 helper 前缀");
    }
    // 仅 cmd/ps/ps7 + maximize + current_exe 可用时前缀 helper
    let helper_exe = if maximize && terminal != "wezterm" {
        match current_exe_quoted() {
            Some(p) => Some(p),
            None => {
                tracing::warn!("[build_spawn_plan] current_exe 失败，退化为不前缀 helper");
                None
            }
        }
    } else {
        None
    };

    match terminal {
        "wezterm" => Ok(SpawnPlan {
            command: "wezterm".to_string(),
            args: [
                vec![
                    "start".to_string(),
                    "--cwd".to_string(),
                    request.working_directory.clone(),
                    "-e".to_string(),
                ],
                process_argv,
            ]
            .concat(),
            current_dir: None,
            creation_flags: Some(DETACHED_PROCESS),
        }),
        "cmd" => {
            let k_arg = match &helper_exe {
                Some(exe) => format!("{} maximize-window && {}", exe, claude_cmdline),
                None => claude_cmdline,
            };
            Ok(SpawnPlan {
                command: "cmd.exe".to_string(),
                args: vec!["/K".to_string(), k_arg],
                current_dir: Some(request.working_directory.clone()),
                creation_flags: Some(CREATE_NEW_CONSOLE),
            })
        }
        "powershell" | "powershell7" => {
            let exe_name = if terminal == "powershell" {
                "powershell.exe"
            } else {
                "pwsh.exe"
            };
            let cmd_arg = match &helper_exe {
                // powershell 用 & 调用操作符包裹带空格/引号的 exe 路径
                Some(exe) => format!("& {} maximize-window; {}", exe, claude_cmdline),
                None => claude_cmdline,
            };
            Ok(SpawnPlan {
                command: exe_name.to_string(),
                args: vec!["-Command".to_string(), cmd_arg],
                current_dir: Some(request.working_directory.clone()),
                creation_flags: Some(CREATE_NEW_CONSOLE),
            })
        }
        other => Err(format!("不支持的终端类型: {}", other)),
    }
}
```

- [ ] **Step 3: 编译验证**

Run: `cd src-tauri && cargo check --message-format=short`
Expected: `Finished` 无错误。

- [ ] **Step 4: 全量测试**

Run: `cd src-tauri && cargo test --lib --message-format=short 2>&1 | tail -5`
Expected: 全部通过（含 Task 2 的 should_skip_ancestor 与既有 launch/settings 测试）。

- [ ] **Step 5: release 构建并手动验证三种终端**

Run: `cd src-tauri && cargo build --release`

手动验证（Windows 默认终端 = WT）：
1. Settings 选 cmd + 勾选最大化 → 新建 session：cmd 窗口应先最大化、claude 全宽渲染。
2. 切 powershell + 最大化 → 新建：同上。
3. 切 powershell7 + 最大化 → 新建：同上。
4. 关闭最大化开关 → 各终端正常启动（无 helper 前缀）。
5. 反复切换+启动：无 error 50、无点击陷阱。
6. resume 一个 session：历史内容全宽渲染、表格不错位。

**若 ps/pwsh 的 `& "exe" maximize-window;` 引号有问题**：回报现象，调整 `current_exe_quoted` 或改用短路径（GetShortPathName）。

- [ ] **Step 6: 提交**

```bash
cd C:/workspace/claude-fleet-sp
git add src-tauri/src/utils/launch/mod.rs
git commit -m "feat(maximize): build_spawn_plan 构造 helper 前缀命令（cmd/ps/ps7），wezterm 跳过"
```

---

## Task 4: 前端 wezterm 时 maximize 开关置灰

**Goal:** SettingsDialog 中终端选 wezterm 时，"打开终端时最大化"开关置灰 + 提示"wezterm 不支持"，与既有 ccglass 置灰模式一致。

**Files:**
- Modify: `src/components/dialogs/SettingsDialog.tsx`

- [ ] **Step 1: 修改 maximize 开关块**

在 `src/components/dialogs/SettingsDialog.tsx` 中，把 231-243 行的 maximize 块：

```tsx
              <div className="flex items-center justify-between rounded-md border p-3">
                <div className="space-y-0.5">
                  <Label htmlFor="maximize-window">打开终端时最大化</Label>
                  <p className="text-xs text-muted-foreground">
                    新建和恢复 session 时将终端窗口自动最大化
                  </p>
                </div>
                <Switch
                  id="maximize-window"
                  checked={launchSettings.maximizeWindow === true}
                  onCheckedChange={(enabled) => updateLaunchSettings({ maximizeWindow: enabled })}
                />
              </div>
```

替换为：

```tsx
              <div className={`flex items-center justify-between rounded-md border p-3 ${terminalType === 'wezterm' ? 'opacity-60' : ''}`}>
                <div className="space-y-0.5">
                  <Label htmlFor="maximize-window">打开终端时最大化</Label>
                  <p className="text-xs text-muted-foreground">
                    新建和恢复 session 时将终端窗口自动最大化（在 claude 渲染前完成）
                  </p>
                  {terminalType === 'wezterm' && (
                    <p className="text-xs text-amber-600">
                      WezTerm 不支持最大化，已自动跳过
                    </p>
                  )}
                </div>
                <Switch
                  id="maximize-window"
                  checked={terminalType === 'wezterm' ? false : launchSettings.maximizeWindow === true}
                  disabled={terminalType === 'wezterm'}
                  onCheckedChange={(enabled) => updateLaunchSettings({ maximizeWindow: enabled })}
                />
              </div>
```

- [ ] **Step 2: 类型检查**

Run: `cd C:/workspace/claude-fleet-sp && npx tsc --noEmit`
Expected: 无错误。

- [ ] **Step 3: 提交**

```bash
cd C:/workspace/claude-fleet-sp
git add src/components/dialogs/SettingsDialog.tsx
git commit -m "feat(maximize): wezterm 时最大化开关置灰 + 提示"
```

---

## Task 5: 移除旧 maximize 机制 + reset_std_handles 还原私有

**Goal:** 删除旧的后台线程最大化代码（`maximize_terminal_window`、`resolve_visible_target_from_pseudo`、`launch_session` 中的线程分支），`reset_std_handles` 还原私有。**保留 `visible_titled_root_owner`**（新 helper 复用）。

**Files:**
- Modify: `src-tauri/src/utils/window_manager.rs`
- Modify: `src-tauri/src/utils/launch/mod.rs`
- Modify: `src-tauri/src/utils/process.rs`

- [ ] **Step 1: 删除 window_manager.rs 中 maximize_terminal_window + resolve_visible_target_from_pseudo**

在 `src-tauri/src/utils/window_manager.rs` 中，删除以下两个函数（连同其上方文档注释）：

1. `pub fn maximize_terminal_window(pid: u32) -> Result<(), String> { ... }`（含其上方的 `/// 启动终端后将其窗口最大化...` 文档注释，到函数结束 `}`）。
2. `fn resolve_visible_target_from_pseudo(pseudo_hwnd: HWND) -> Option<HWND> { ... }`（含其上方 `/// 从 console pseudo HWND 解析...` 文档注释）。

**保留** `fn visible_titled_root_owner(hwnd: HWND) -> Option<HWND>`（Task 1 的 `resolve_console_target` 复用它）。

同时删除文件底部非 Windows 平台的 `maximize_terminal_window` stub：

```rust
#[cfg(not(target_os = "windows"))]
pub fn maximize_terminal_window(_pid: u32) -> Result<(), String> {
    Err("仅支持 Windows 平台".to_string())
}
```

- [ ] **Step 2: launch_session 删除后台线程最大化分支**

在 `src-tauri/src/utils/launch/mod.rs` 中，把 `pub fn launch_session`（原 168-184 行）：

```rust
pub fn launch_session(request: &LaunchRequest) -> Result<(), String> {
    let plan = build_spawn_plan(request)?;
    let child = spawn_plan(&plan)?;

    if request.settings.maximize_window {
        let pid = child.id();
        tracing::info!("[launch_session] maximize_window=true，后台最大化终端窗口 pid={}", pid);
        // 后台轮询定位并最大化，不阻塞启动返回；丢 child 不影响终端进程存活
        std::thread::spawn(move || {
            if let Err(e) = crate::utils::window_manager::maximize_terminal_window(pid) {
                tracing::warn!("[launch_session] 最大化终端窗口失败 pid={}: {}", pid, e);
            }
        });
    }

    Ok(())
}
```

替换为：

```rust
pub fn launch_session(request: &LaunchRequest) -> Result<(), String> {
    let plan = build_spawn_plan(request)?;
    // 最大化由终端命令前置的 helper 子命令完成（build_spawn_plan 在 maximize_window=true
    // 时已构造 "<helper> maximize-window && claude..."），此处不再事后补最大化。
    // 丢 child 不影响终端进程存活（drop 仅关闭句柄，不杀进程）。
    let _child = spawn_plan(&plan)?;
    Ok(())
}
```

- [ ] **Step 3: process.rs 还原 reset_std_handles 为私有**

在 `src-tauri/src/utils/process.rs` 中，把 `reset_std_handles` 的签名与文档注释从：

```rust
/// 清理当前进程的标准句柄：分离控制台 + 将三个标准句柄置 NULL。
///
/// best-effort：任一调用失败均忽略，下一步 spawn 会重新探测句柄状态。
///
/// pub(crate)：供 window_manager::maximize_terminal_window 在 attach 序列后主动调用——
/// AttachConsole 把本进程挂到子进程 console，FreeConsole 后 std handle 残留为失效句柄，
/// 后续 CreateProcess(CREATE_NEW_CONSOLE) 会报 os error 50 (ERROR_NOT_SUPPORTED)。
#[cfg(target_os = "windows")]
pub(crate) fn reset_std_handles() {
```

改回：

```rust
/// 清理当前进程的标准句柄：分离控制台 + 将三个标准句柄置 NULL。
///
/// best-effort：任一调用失败均忽略，下一步 spawn 会重新探测句柄状态。
#[cfg(target_os = "windows")]
fn reset_std_handles() {
```

- [ ] **Step 4: 编译验证**

Run: `cd src-tauri && cargo check --message-format=short`
Expected: `Finished` 无错误、无 `unused` 警告。

- [ ] **Step 5: 全量测试**

Run: `cd src-tauri && cargo test --lib --message-format=short 2>&1 | tail -5`
Expected: 全部通过。

- [ ] **Step 6: 提交**

```bash
cd C:/workspace/claude-fleet-sp
git add src-tauri/src/utils/window_manager.rs src-tauri/src/utils/launch/mod.rs src-tauri/src/utils/process.rs
git commit -m "refactor(maximize): 移除旧后台线程最大化机制，reset_std_handles 还原私有"
```

---

## Task 6: 全量验证 + 最终构建

**Goal:** 完整手动验证清单 + 产出最终 release exe。

**Files:** 无修改（仅验证与构建）。

- [ ] **Step 1: 全量 cargo test**

Run: `cd src-tauri && cargo test --message-format=short 2>&1 | tail -8`
Expected: 全部通过。

- [ ] **Step 2: 前端类型检查**

Run: `cd C:/workspace/claude-fleet-sp && npx tsc --noEmit`
Expected: 无错误。

- [ ] **Step 3: 最终 release 构建**

Run: `cd C:/workspace/claude-fleet-sp && npm run tauri build`
Expected: 产出 `src-tauri/target/release/claude-fleet.exe` 与 NSIS 安装包。

- [ ] **Step 4: 手动验证清单（运行新 exe）**

运行 `src-tauri/target/release/claude-fleet.exe`，逐项确认：

- [ ] cmd + 最大化 + WT 宿主 → 真最大化、claude 全宽、无 error 50、无陷阱
- [ ] powershell + 最大化 + WT 宿主 → 同上
- [ ] powershell7 + 最大化 + WT 宿主 → 同上
- [ ] cmd + 最大化 + 经典 conhost（关闭 WT 默认终端） → 真最大化
- [ ] powershell + 最大化 + 经典 conhost → 真最大化
- [ ] powershell7 + 最大化 + 经典 conhost → 真最大化
- [ ] wezterm + 最大化 → 直接启动、开关置灰、warn 跳过
- [ ] 关闭最大化开关 → 各终端正常直接启动
- [ ] 反复切换终端+开关+启动 → 始终无 error 50、无陷阱
- [ ] resume session → 历史内容全宽渲染、表格不错位

- [ ] **Step 5: 最终提交（如有验证中发现的修复）**

若 Step 4 全部通过则无需额外提交；若发现并修复了小问题，提交：

```bash
cd C:/workspace/claude-fleet-sp
git add -A
git commit -m "fix(maximize): 验证中发现的小修复"
```

---

## 自检（plan vs spec 覆盖）

- spec §架构总览 → Task 1+3（helper + spawn_plan 前缀）
- spec §helper 4 步逻辑 → Task 1（步骤1-2）+ Task 2（步骤3，should_skip_ancestor）
- spec §spawn_plan 表 → Task 3（cmd/ps/ps7 前缀、wezterm 跳过）
- spec §wezterm 约束（前端置灰） → Task 4
- spec §移除旧代码 → Task 5（maximize_terminal_window / resolve_visible_target_from_pseudo / launch_session 线程分支 / reset_std_handles 还原）
- spec §错误处理/降级 → Task 3 Step 2（current_exe 失败退化、helper 失败 exit 0 在 main.rs Task1 Step1）
- spec §测试 → Task 2（should_skip_ancestor 单测）+ Task 6（手动清单）
- spec §实现顺序含 spike → Task 1 Step 6（失败立即回报门槛）
- spec §核心假设（AttachConsole+GetAncestor） → Task 1 Step 6 spike 验证

修正：spec §移除旧代码 列了 `visible_titled_root_owner`，但新 helper 的 `resolve_console_target` 复用它 → 本计划**保留** `visible_titled_root_owner`，仅删 `maximize_terminal_window` 与 `resolve_visible_target_from_pseudo`。
