# 已知问题

本文档记录 Claude Fleet 当前版本的已知限制和问题。

## 1. Windows Terminal 跳转支持

**状态**：✅ 已支持（v0.8.3，方案见 `docs/2026-07-20-windows-terminal-jump-design.md`）

**原理**：Windows Terminal 所有 tab 共用同一个 `WindowsTerminal.exe` 进程，原"从 claude PID 逐层向上找持有窗口的父进程"的思路只能命中 WT 主进程，无法区分具体 tab。现改用 `AttachConsole(claude_pid) + GetConsoleWindow()` 直接拿到该 tab 对应的不可见 pseudo-console 宿主窗口（per-tab 唯一），再 `SetForegroundWindow` 该 HWND，WT v1.14+ 会把操作传播到真实主窗口并切到正确 tab。

**非 WT 终端不受影响**：cmd / PowerShell / WezTerm / Git Bash 仍走原父链逻辑（实现里通过 owner 进程名识别 WindowsTerminal，仅 WT 走新路径）。

**前置要求**：Windows Terminal ≥ 1.14（2022 年起所有正式版均满足）。提权场景（claude 以管理员起、Claude Fleet 以普通用户起）下 `AttachConsole` 会被拒绝，回退父链（仅激活主窗口、不切 tab）。

**替代方案**：仍可使用 WezTerm、cmd 或 PowerShell 作为终端。

---

## 2. Session 启动时无法命名

**状态**：~~已解决~~ (v0.3.0)

Session 启动时的名称功能存在问题，当前不支持对 session 进行额外命名或展示自定义名称的能力。

**解决方案**：已在 v0.3.0 版本中实现新建 session 时支持自定义名称。用户可以在创建 session 时指定名称，该名称会显示在 session 列表和详情中。

---

## 3. 仅支持 64 位 Windows

**状态**：平台限制

当前仅支持和验证过 64 位 Windows 环境。开发团队没有 Linux 和 Mac 环境，无法验证其他平台的兼容性。

**解决方案**：在项目定位调整之前，不需要考虑除 64 位 Windows 以外的运行环境。

---

## 4. 后台运行 Shell 时状态不变为等待输入

**状态**：Claude Code 行为限制

当 Claude Code 有运行中的 shell 命令时（即使该 shell 处于后台运行状态），Claude Code 的状态不会变为 `idle` 或 `waiting`。这导致：

- Claude Fleet 无法正确识别该 session 为"等待输入"状态
- away_summary 摘要功能在此情况下不会触发显示

**原因**：Claude Code 的状态判定机制将后台运行的 shell 视为活动状态。

**解决方案**：暂无。这是 Claude Code 的内部行为，Claude Fleet 无法干预。

---

## 5. Git Bash 终端无法支持跳转

**状态**：暂不支持

**原因**：技术限制

在 Windows 环境下，Git Bash 终端无法支持跳转功能。原因是 Git Bash 在启动链中使用某种临时进程（该进程会立刻销毁），导致无法根据父进程链找到持有窗口的进程。

**解决方案**：暂无。

**替代方案**：使用 WezTerm、cmd 或 PowerShell 作为终端。