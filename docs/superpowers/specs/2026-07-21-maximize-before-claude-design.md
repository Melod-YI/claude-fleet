# 打开终端时最大化：渲染前最大化重构设计

日期：2026-07-21
状态：已批准（待实现）

## 背景与问题

Claude Fleet 有"打开终端时最大化"开关（`LaunchSettings.maximize_window`）。当前实现：`launch_session` spawn 终端后，后台线程轮询 `maximize_terminal_window(pid)`，通过 `AttachConsole(child_pid) + GetConsoleWindow()` 拿到 console 窗口再 `ShowWindow(SW_MAXIMIZE)`。

该实现存在三个问题：

1. **点击陷阱**：`ShowWindow(SW_MAXIMIZE)`（== `SW_SHOWMAXIMIZED`）对不可见的 WT/ConPTY pseudo-console 宿主窗口会"激活 + 铺满全屏"形成整屏隐形前台窗口，吞掉所有鼠标点击，而可见终端主窗口保持原大小。
2. **os error 50**：后台线程在 Tauri 主进程内调用 `AttachConsole/FreeConsole`，残留失效标准句柄，后续 `CreateProcess(CREATE_NEW_CONSOLE)` 报 `ERROR_NOT_SUPPORTED`（50），表现为"首次启动正常，之后所有启动都失败"。
3. **时序错位**：`cmd /K "claude..."` 一开窗口 claude 就立刻渲染。后台最大化必然晚于渲染。对**恢复 session** 影响显著：resume 时 claude 重新渲染历史对话，若窗口在小尺寸下开始渲染、中途才最大化，已渲染内容（表格、宽内容）不适配新宽度，出现错位。

## 目标

- 最大化必须在 **claude 开始渲染之前**完成（硬需求，覆盖新建与恢复）。
- 覆盖 cmd / powershell / powershell7（含 WT 宿主与经典 conhost 两种情形）。
- wezterm 不支持最大化（约束，与 ccglass 一致地"不支持 wezterm"）。
- 移除 Tauri 主进程内的 `AttachConsole` 调用，根除 error 50 与点击陷阱隐患。

## 方案：helper 子命令 + 终端命令前置最大化

把"最大化"从**后台线程事后补**改为**终端启动命令的前置步骤**。

### 核心思路

新增 CLI 子命令 `claude-fleet.exe maximize-window`：在 helper 自己的进程内最大化当前/父终端窗口后 `exit 0`。helper 进程短命，其 `AttachConsole` 污染随进程退出消亡，Tauri 主进程永不调用 `AttachConsole`。

`maximize_window=true` 时，`build_spawn_plan` 把终端要执行的 claude 命令**前缀** `"<helper> maximize-window"`，用 `&&`（cmd）/ `;`（powershell）串联：helper 先跑完最大化，终端再启动 claude → **最大化在 claude 渲染前完成**。

`maximize_window=false` 时维持现有直接启动形态，零变化。

### helper 路径解析

helper 路径用 `std::env::current_exe()` 取 claude-fleet.exe 绝对路径并加引号。`current_exe()` 失败（极罕见）时该次启动退化为直接跑 claude（不最大化），记 warn。

### helper 最大化逻辑 `maximize_current_process_window()`

1. `AttachConsole(ATTACH_PARENT_PROCESS)`（挂到父进程 = 终端进程的 console）。
2. 若成功且 `GetConsoleWindow()` 非空：解析【可见且有非空标题】的目标——
   - WT：`GetAncestor(GA_ROOTOWNER)` 取宿主 WT 主窗口；
   - conhost / cmd 自持：直接用该 console 窗口；
   - 调 `find_window_by_pid(owner_pid)` 兜底（要求可见+有标题，过滤 pseudo）。
   - 拿到可见目标即 `ShowWindow(SW_MAXIMIZE)`，跳过步骤 3。
3. 若步骤 2 拿不到可见目标（wezterm ConPTY 等）：沿父链 `find_window_by_pid_chain` 找第一个可见+有标题的祖先窗口，但**跳过 image 名含 `claude-fleet` 的祖先**（防误最大化 app 自身窗口）→ `ShowWindow`。
4. **始终 exit 0**：最大化是 best-effort，绝不阻塞 claude 启动。

#### 为什么比旧方案可靠

- helper 在 `cmd /K` 内执行，此时 console / WT 主窗口 / owner 链**早已建立**（非 spawn 后 1.5s 抢跑），无时序竞争。
- helper 父链终点是终端进程而非 Claude Fleet：cmd/WT 链 helper→cmd→Claude Fleet（步骤 2 命中 WT 主窗口，不到步骤 3）；wezterm 链 helper→cmd→wezterm（步骤 3 在到达 Claude Fleet 前命中 wezterm 窗口；但 wezterm 已不支持最大化，见下）。

### spawn_plan 改造（maximize_window=true 分支）

| 终端 | 命令形态 |
|---|---|
| cmd | `cmd.exe /K "<helper> maximize-window && <claude命令行>"` + cwd + `CREATE_NEW_CONSOLE` |
| powershell | `powershell.exe -Command "<helper> maximize-window; <claude命令行>"` + cwd + `CREATE_NEW_CONSOLE` |
| powershell7 | `pwsh.exe -Command "<helper> maximize-window; <claude命令行>"` + cwd + `CREATE_NEW_CONSOLE` |
| wezterm | **不支持**——维持 `wezterm start --cwd <wd> -e <process_argv>`，`warn!` 跳过 |

`<claude命令行>` = 现有 `command_line(process_argv)`（含 wrapper，wezterm 仍跳过 wrapper）。

### wezterm 约束

- 后端：`maximize_window=true` + `terminal_id=wezterm` 时直接按现有形态启动，不加 helper 前缀，`warn!` 日志"wezterm 不支持最大化，已跳过"。
- 前端：SettingsDialog 中当终端选 wezterm 时，"打开终端时最大化"开关置灰 + 提示"wezterm 不支持"。
- 备注：wezterm 为 wt 跳转不可用时的权宜方案，wt 跳转已解决，后续剥离 wezterm 时一并清理此分支与对应不支持判断。

## 移除的旧代码

- `window_manager.rs`：`maximize_terminal_window`、`resolve_visible_target_from_pseudo`、`visible_titled_root_owner`（仅 maximize 用；`find_window_by_pid` / `find_window_by_pid_chain` / `is_windows_terminal_window` / `get_process_image_basename` 等被跳转等他处复用，保留）。
- `launch/mod.rs`：`launch_session` 中 `std::thread::spawn(maximize_terminal_window)` 分支删除。
- `process.rs`：`reset_std_handles` 还原为私有 `fn`（之前为 maximize 调用改的 `pub(crate)` 不再需要；`spawn` 的 error 6 恢复仍内部使用）。

## 新增代码

- CLI 子命令入口（`main.rs`）：`argv[1] == "maximize-window"` 时调 `maximize_current_process_window()` 后 `exit 0`，跳过 Tauri 初始化。
- `window_manager.rs`：`maximize_current_process_window()`（上述 4 步逻辑）+ 纯决策 `should_skip_ancestor(image_name: &str) -> bool`（含 `claude-fleet` 即跳过）。
- `launch/mod.rs`：`build_spawn_plan` 在 `maximize_window=true` 时为 cmd/ps/ps7 构造 helper 前缀命令；wezterm 分支加 warn 跳过。

## 错误处理 / 降级

- `current_exe()` 失败：该次退化为直接跑 claude（不最大化），warn。
- helper 内任一步失败：记日志，仍 exit 0，claude 照常启动。
- helper 找不到任何可见窗口：exit 0，无最大化、无副作用。

## 测试

- 纯决策单测：`should_skip_ancestor("claude-fleet.exe")` == true、`should_skip_ancestor("WindowsTerminal.exe")` == false 等，锁定"绝不最大化 app 自身"不变量。
- 复用既有 `find_window_by_pid`（已要求可见+有标题）的覆盖。
- Win32 interop 主路径无法纯单测（与既有 window_manager 风格一致），靠手动验证清单：
  - cmd / powershell / powershell7 各在 WT 宿主与经典 conhost 下，开关 maximize 反复启动 → 真最大化 + 无 error 50 + 无点击陷阱 + resume 不错位。
  - wezterm 开 maximize → 直接启动、warn 跳过、开关置灰。
  - 恢复 session → 历史内容全宽渲染、表格不错位。

## 实现顺序（含验证 spike）

1. **最小 helper spike**：落 CLI 子命令 + `maximize_current_process_window()`（步骤 2 为主），手动在 cmd + WT 宿主下跑 `claude-fleet.exe maximize-window`，确认 WT 真最大化、无 error 50、无陷阱。**失败则立刻回报，不继续。**
2. 扩展步骤 3 父链兜底 + `should_skip_ancestor` + 单测。
3. `build_spawn_plan` 改造（cmd/ps/ps7 helper 前缀；wezterm warn 跳过）。
4. 前端 SettingsDialog：wezterm 时 maximize 开关置灰。
5. 移除旧 `maximize_terminal_window` 等死代码 + `reset_std_handles` 还原私有。
6. 全量手动验证清单。

## 风险

- **核心假设**：`AttachConsole(ATTACH_PARENT_PROCESS) + GetAncestor` 从 helper 内能拿到 WT 主窗口。比旧 Tauri 侧 attach 更可靠（无时序竞争），但未实测——由实现步骤 1 spike 验证。
- wezterm 已明确不支持最大化，无 PTY/ConPTY 风险。
