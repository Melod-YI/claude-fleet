# 工作目录悬浮增强实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在工作目录悬浮组件中添加"打开目录"和"打开VSCode"按钮，并统一应用到 SessionDetail 和 SessionCard。

**Architecture:** 后端新增两个 Tauri 命令，前端服务层封装调用，创建共享组件 PathHoverDisplay 统一两处使用。

**Tech Stack:** Rust (Tauri), TypeScript, React, Tailwind CSS, lucide-react

---

## 文件结构

| 文件 | 变更类型 | 职责 |
|------|----------|------|
| `src-tauri/src/commands/terminal.rs` | 修改 | 新增 `open_directory`、`open_in_vscode` 命令 |
| `src-tauri/src/lib.rs` | 修改 | 注册新命令到 invoke_handler |
| `src/services/terminalService.ts` | 修改 | 封装 `openDirectory`、`openInVSCode` 函数 |
| `src/services/index.ts` | 修改 | 导出新函数 |
| `src/components/common/PathHoverDisplay.tsx` | 新增 | 共享悬浮路径组件 |
| `src/components/management/SessionDetail.tsx` | 修改 | 替换现有路径悬浮实现 |
| `src/components/running/SessionCard.tsx` | 修改 | 替换 cwd 显示 |

---

## Task 1: 后端 - 新增打开目录命令

**Files:**
- Modify: `src-tauri/src/commands/terminal.rs`

- [ ] **Step 1: 添加必要的 import**

在文件顶部添加 `std::process::Command`：

```rust
use std::process::Command;
use crate::utils::window_manager::{
    find_terminal_window,
    find_window_by_pid_chain,
    activate_window,
    start_terminal_with_resume,
};
use tracing::{info, debug, warn, error};
```

- [ ] **Step 2: 添加 open_directory 命令**

在文件末尾添加：

```rust
/// 打开目录（Windows 用 explorer）
#[tauri::command]
pub fn open_directory(path: String) -> Result<(), String> {
    info!("[open_directory] 开始，路径: {}", path);

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
        info!("[open_directory] 完成");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("仅支持 Windows 平台".to_string())
    }
}
```

- [ ] **Step 3: 添加 open_in_vscode 命令**

继续在文件末尾添加：

```rust
/// 在 VSCode 中打开目录
#[tauri::command]
pub fn open_in_vscode(path: String) -> Result<(), String> {
    info!("[open_in_vscode] 开始，路径: {}", path);

    #[cfg(target_os = "windows")]
    {
        Command::new("code")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("打开 VSCode 失败: {}。请确保 VSCode 已安装且 'code' 命令在 PATH 中", e))?;
        info!("[open_in_vscode] 完成");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        Err("仅支持 Windows 平台".to_string())
    }
}
```

- [ ] **Step 4: 提交变更**

```bash
git add src-tauri/src/commands/terminal.rs
git commit -m "feat: 新增 open_directory 和 open_in_vscode 命令"
```

---

## Task 2: 后端 - 注册新命令

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 添加 import**

修改第 23 行，添加新命令到 import：

```rust
use commands::terminal::{jump_to_terminal, jump_to_terminal_by_pid, smart_jump_to_terminal, resume_in_terminal, open_directory, open_in_vscode};
```

- [ ] **Step 2: 注册命令到 invoke_handler**

修改第 88-115 行，在 `resume_in_terminal` 后添加：

```rust
.invoke_handler(tauri::generate_handler![
    // New optimized session commands for management tab
    list_sessions_optimized,
    get_session_messages_optimized,
    delete_session_optimized,
    // Running session commands (keep for Running Tab)
    init_running,
    list_running,
    start_polling_cmd,
    stop_polling_cmd,
    // Legacy commands (keep for compatibility)
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_sessions_watcher,
    stop_sessions_watcher,
    start_hooks,
    stop_hooks,
    delete_session_cmd,
    // Terminal commands
    jump_to_terminal,
    jump_to_terminal_by_pid,
    smart_jump_to_terminal,
    resume_in_terminal,
    open_directory,
    open_in_vscode,
    // Sound commands
    get_available_sounds,
    get_sound_data
])
```

- [ ] **Step 3: 提交变更**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: 注册 open_directory 和 open_in_vscode 命令"
```

---

## Task 3: 后端 - 验证编译

- [ ] **Step 1: 检查 Rust 编译**

```bash
cd src-tauri && cargo check
```

Expected: 无编译错误

- [ ] **Step 2: 如有错误则修复**

如果出现 import 错误，检查路径是否正确。

---

## Task 4: 前端 - 添加服务函数

**Files:**
- Modify: `src/services/terminalService.ts`

- [ ] **Step 1: 添加 openDirectory 函数**

在文件末尾（`resumeInTerminal` 函数后）添加：

```typescript
/**
 * 打开目录（Windows 资源管理器）
 */
export async function openDirectory(path: string): Promise<void> {
  try {
    await invoke('open_directory', { path })
  } catch (error) {
    throw new Error(String(error))
  }
}

/**
 * 在 VSCode 中打开目录
 */
export async function openInVSCode(path: string): Promise<void> {
  try {
    await invoke('open_in_vscode', { path })
  } catch (error) {
    throw new Error(String(error))
  }
}
```

- [ ] **Step 2: 提交变更**

```bash
git add src/services/terminalService.ts
git commit -m "feat: 新增 openDirectory 和 openInVSCode 服务函数"
```

---

## Task 5: 前端 - 导出新函数

**Files:**
- Modify: `src/services/index.ts`

- [ ] **Step 1: 确认导出**

`terminalService.ts` 已在 index.ts 中导出：`export * from './terminalService'`，新函数会自动导出。

无需修改 index.ts。

---

## Task 6: 前端 - 创建共享组件

**Files:**
- Create: `src/components/common/PathHoverDisplay.tsx`

- [ ] **Step 1: 创建组件文件**

```tsx
import { useState } from "react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { FolderOpen, Copy, Code, Check } from "lucide-react"
import { openDirectory, openInVSCode } from "@/services"
import { toast } from "sonner"

interface PathHoverDisplayProps {
  path: string           // 完整路径
  displayName?: string   // 显示名称（默认取路径最后一段）
  className?: string     // 外层容器样式
}

export function PathHoverDisplay({ path, displayName, className }: PathHoverDisplayProps) {
  const [copied, setCopied] = useState(false)

  const displayText = displayName || path.split(/[\\/]/).filter(Boolean).pop() || path

  const handleCopy = async () => {
    await navigator.clipboard.writeText(path)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleOpenDirectory = async () => {
    try {
      await openDirectory(path)
    } catch (error) {
      toast.error(String(error))
    }
  }

  const handleOpenVSCode = async () => {
    try {
      await openInVSCode(path)
    } catch (error) {
      toast.error(String(error))
    }
  }

  return (
    <div className={cn("relative group min-w-0", className)}>
      {/* 基础显示 */}
      <div className="flex items-center gap-1.5 cursor-default">
        <FolderOpen className="w-4 h-4 shrink-0 text-gray-400" />
        <span className="truncate text-sm text-gray-600">{displayText}</span>
      </div>

      {/* 悬浮层 */}
      <div className="absolute left-0 top-full mt-1 hidden group-hover:flex items-center gap-2 bg-white border border-gray-200 rounded-md px-3 py-1.5 shadow-md z-10 min-w-[200px]">
        <span className="text-xs text-gray-600 truncate max-w-[300px]" title={path}>{path}</span>
        <div className="flex items-center gap-1 shrink-0">
          {/* 复制 */}
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopy}
            className="p-0.5 h-auto"
            title="复制路径"
          >
            {copied ? (
              <Check className="w-3 h-3 text-green-500" />
            ) : (
              <Copy className="w-3 h-3" />
            )}
          </Button>
          {/* 打开目录 */}
          <Button
            variant="ghost"
            size="sm"
            onClick={handleOpenDirectory}
            className="p-0.5 h-auto"
            title="打开目录"
          >
            <FolderOpen className="w-3 h-3" />
          </Button>
          {/* 打开 VSCode */}
          <Button
            variant="ghost"
            size="sm"
            onClick={handleOpenVSCode}
            className="p-0.5 h-auto"
            title="在 VSCode 中打开"
          >
            <Code className="w-3 h-3" />
          </Button>
        </div>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: 提交变更**

```bash
git add src/components/common/PathHoverDisplay.tsx
git commit -m "feat: 创建 PathHoverDisplay 共享组件"
```

---

## Task 7: 前端 - 集成到 SessionDetail

**Files:**
- Modify: `src/components/management/SessionDetail.tsx`

- [ ] **Step 1: 添加 import**

在第 9 行的 import 中添加 `PathHoverDisplay`：

```typescript
import { Star, Trash2, Copy, Check, RefreshCw, Play, Clock, FolderOpen } from "lucide-react"
import { formatRelativeTime } from "@/utils"
import { PathHoverDisplay } from "@/components/common/PathHoverDisplay"
```

- [ ] **Step 2: 替换路径悬浮实现**

替换第 116-137 行的路径悬浮代码：

原代码（删除）：
```tsx
{/* 路径 */}
{session.projectDir && (
  <div className="relative group min-w-0 max-w-[400px]">
    <div className="flex items-center gap-1.5 cursor-default">
      <FolderOpen className="w-4 h-4 shrink-0" />
      <span className="truncate">
        {session.projectDir.split(/[\\/]/).filter(Boolean).pop() || session.projectDir}
      </span>
    </div>
    {/* 悬浮显示完整路径 */}
    <div className="absolute left-0 top-full mt-1 hidden group-hover:flex items-center gap-2 bg-white border border-gray-200 rounded-md px-3 py-1.5 shadow-md z-10 max-w-[500px]">
      <span className="text-xs text-gray-600 truncate">{session.projectDir}</span>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => navigator.clipboard.writeText(session.projectDir || "")}
        className="p-0.5 h-auto shrink-0"
      >
        <Copy className="w-3 h-3" />
      </Button>
    </div>
  </div>
)}
```

替换为：
```tsx
{/* 路径 */}
{session.projectDir && (
  <PathHoverDisplay path={session.projectDir} className="max-w-[400px]" />
)}
```

- [ ] **Step 3: 提交变更**

```bash
git add src/components/management/SessionDetail.tsx
git commit -m "feat: SessionDetail 使用 PathHoverDisplay 组件"
```

---

## Task 8: 前端 - 集成到 SessionCard

**Files:**
- Modify: `src/components/running/SessionCard.tsx`

- [ ] **Step 1: 添加 import**

在第 7 行的 import 后添加：

```typescript
import { Star, Folder, Clock } from "lucide-react"
import { PathHoverDisplay } from "@/components/common/PathHoverDisplay"
```

注意：`Folder` 图标可能不再需要，但保留以防其他地方使用。

- [ ] **Step 2: 替换 cwd 显示**

替换第 149-155 行的 cwd 显示代码：

原代码（删除）：
```tsx
<span
  className="flex items-center gap-1 truncate max-w-[200px]"
  title={session.cwd}
>
  <Folder className="w-3 h-3 text-gray-400 shrink-0" />
  {session.cwd.split(/[\\/]/).filter(Boolean).pop() || session.cwd}
</span>
```

替换为：
```tsx
<PathHoverDisplay
  path={session.cwd}
  className="max-w-[200px]"
/>
```

- [ ] **Step 3: 提交变更**

```bash
git add src/components/running/SessionCard.tsx
git commit -m "feat: SessionCard 使用 PathHoverDisplay 组件"
```

---

## Task 9: 验证 - 前端编译检查

- [ ] **Step 1: 检查 TypeScript 编译**

```bash
npm run build
```

Expected: 无编译错误

- [ ] **Step 2: 如有错误则修复**

检查 import 路径是否正确，组件 props 是否匹配。

---

## Task 10: 验证 - 启动开发模式

- [ ] **Step 1: 启动 Tauri 开发模式**

```bash
npm run tauri dev
```

Expected: 应用正常启动

- [ ] **Step 2: 手动验证功能**

1. 打开 Session 管理页，选中一个 session
2. 悬浮路径显示完整路径 + 三个按钮
3. 点击复制按钮，验证路径复制成功
4. 点击打开目录按钮，验证资源管理器打开
5. 点击打开 VSCode 按钮，验证 VSCode 打开目录
6. 打开运行中页面，验证卡片悬浮同样功能

---

## Task 11: 提交最终变更

- [ ] **Step 1: 确认所有变更已提交**

```bash
git status
```

Expected: 无未提交的变更

- [ ] **Step 2: 推送到远程（可选）**

```bash
git push origin master
```