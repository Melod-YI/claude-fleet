# 已知问题

本文档记录 Claude Fleet 当前版本的已知限制和问题。

## 1. Windows Terminal 不受支持

**状态**：暂不支持

**原因**：技术限制

在 Windows 环境下，无法实现跳转到 Windows Terminal 的指定窗口。原因是所有 Windows Terminal 窗口共用同一个进程 PID，从 Claude 进程的 PID 出发无法定位到具体的窗口信息。

**解决方案**：暂无。在获得更好的解决方式前，所有情况都不考虑使用 Windows Terminal。

**替代方案**：使用 WezTerm、cmd 或 PowerShell 作为终端。

---

## 2. Session 启动时无法命名

**状态**：功能缺失

Session 启动时的名称功能存在问题，当前不支持对 session 进行额外命名或展示自定义名称的能力。

**解决方案**：后续版本考虑添加。

---

## 3. 仅支持 64 位 Windows

**状态**：平台限制

当前仅支持和验证过 64 位 Windows 环境。开发团队没有 Linux 和 Mac 环境，无法验证其他平台的兼容性。

**解决方案**：在项目定位调整之前，不需要考虑除 64 位 Windows 以外的运行环境。