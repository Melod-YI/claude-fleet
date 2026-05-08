# 终端启动命令可配置实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将恢复 session 时使用的终端从硬编码改为用户可配置，支持 wezterm、cmd、powershell 三种选择。

**Architecture:** 前端枚举选择终端类型，后端根据类型匹配预设参数模板执行。类型定义和设置 Store 存储用户选择，后端 window_manager.rs 实现各终端启动逻辑。

**Tech Stack:** TypeScript (React), Zustand, Tauri 2.0 (Rust), shadcn/ui

---

## 文件结构

| 文件 | 责任 |
|------|------|
| `src/types/settings.ts` | 定义 `TerminalType` 类型，修改 `AppSettings` 接口 |
| `src/stores/settingsStore.ts` | 添加默认值和 `setTerminalType` 方法 |
| `src-tauri/src/utils/window_manager.rs` | 终端配置结构体，参数模板，修改 `start_terminal_with_resume` |
| `src-tauri/src/commands/terminal.rs` | `resume_in_terminal` 命令增加终端类型参数 |
| `src/services/terminalService.ts` | 调用时传递终端类型 |
| `src/components/dialogs/SettingsDialog.tsx` | 新建：设置对话框，包含终端选择 |
| `src/components/layout/AppLayout.tsx` | 添加设置按钮触发对话框 |

---

### Task 1: 前端类型定义

**Files:**
- Modify: `src/types/settings.ts:1-18`

- [ ] **Step 1: 添加 TerminalType 类型定义**

在文件顶部添加：

```typescript
export type TerminalType = 'wezterm' | 'cmd' | 'powershell'
```

- [ ] **Step 2: 修改 AppSettings 接口**

在 `AppSettings` 接口中添加 `terminalType` 字段：

```typescript
export interface AppSettings {
  favoritePaths: FavoritePaths
  defaultTimeRange: '3d' | '7d' | '30d' | 'all'
  notificationSound: boolean
  notificationDesktop: boolean
  theme: 'light' | 'dark' | 'system'
  terminalType: TerminalType  // 新增
}
```

---

### Task 2: 前端设置 Store

**Files:**
- Modify: `src/stores/settingsStore.ts:19-25`
- Modify: `src/stores/settingsStore.ts:80-140`

- [ ] **Step 1: 更新 DEFAULT_SETTINGS**

在 `DEFAULT_SETTINGS` 中添加 `terminalType` 默认值：

```typescript
const DEFAULT_SETTINGS: AppSettings = {
  favoritePaths: { paths: [] },
  defaultTimeRange: '30d',
  notificationSound: true,
  notificationDesktop: true,
  theme: 'system',
  terminalType: 'wezterm',  // 新增
}
```

- [ ] **Step 2: 添加 setTerminalType 方法**

在 `SettingsState` 接口中添加：

```typescript
interface SettingsState extends AppSettings {
  // 现有方法...
  recordPathUsage: (path: string) => void
  removeFavoritePath: (path: string) => void
  setDefaultTimeRange: (range: '3d' | '7d' | '30d' | 'all') => void
  setNotificationSound: (enabled: boolean) => void
  setNotificationDesktop: (enabled: boolean) => void
  setTheme: (theme: 'light' | 'dark' | 'system') => void
  setTerminalType: (type: TerminalType) => void  // 新增
  getSortedFavoritePaths: () => FavoritePath[]
}
```

- [ ] **Step 3: 实现 setTerminalType 方法**

在 Store 实现中添加：

```typescript
setTerminalType: (type) => set({ terminalType: type }),
```

放在 `setTheme` 之后，`getSortedFavoritePaths` 之前。

---

### Task 3: 后端终端配置结构

**Files:**
- Modify: `src-tauri/src/utils/window_manager.rs:407-468`

- [ ] **Step 1: 定义 TerminalConfig 结构体**

在文件顶部 `use` 语句之后添加：

```rust
/// 终端配置结构
struct TerminalConfig {
    command: &'static str,
    args: Vec<&'static str>,
}

/// 获取终端配置
fn get_terminal_config(terminal_type: &str) -> Option<TerminalConfig> {
    match terminal_type {
        "wezterm" => Some(TerminalConfig {
            command: "wezterm",
            args: vec![
                "start",
                "--cwd", "{cwd}",
                "-e", "claude",
                "--resume", "{session_id}",
                "--permission-mode", "bypassPermissions",
            ],
        }),
        "cmd" => Some(TerminalConfig {
            command: "cmd.exe",
            args: vec![
                "/K",
                "claude --resume {session_id} --permission-mode bypassPermissions",
            ],
        }),
        "powershell" => Some(TerminalConfig {
            command: "powershell.exe",
            args: vec![
                "-NoExit",
                "-Command",
                "claude --resume {session_id} --permission-mode bypassPermissions",
            ],
        }),
        _ => None,
    }
}
```

- [ ] **Step 2: 修改 start_terminal_with_resume 函数签名**

将函数签名从：

```rust
pub fn start_terminal_with_resume(working_directory: &str, session_id: &str) -> Result<(), String>
```

改为：

```rust
pub fn start_terminal_with_resume(working_directory: &str, session_id: &str, terminal_type: &str) -> Result<(), String>
```

- [ ] **Step 3: 重写 start_terminal_with_resume 实现**

替换整个函数实现：

```rust
pub fn start_terminal_with_resume(working_directory: &str, session_id: &str, terminal_type: &str) -> Result<(), String> {
    info!("[start_terminal_with_resume] 开始启动终端，工作目录: {}, session_id: {}, 终端: {}",
          working_directory, session_id, terminal_type);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;

        // 获取终端配置
        let config = match get_terminal_config(terminal_type) {
            Some(c) => c,
            None => {
                error!("[start_terminal_with_resume] 不支持的终端类型: {}", terminal_type);
                return Err(format!("不支持的终端类型: {}", terminal_type));
            }
        };

        info!("[start_terminal_with_resume] 终端配置: {} {}", config.command, config.args.join(" "));

        // 替换参数中的变量
        let args: Vec<String> = config.args.iter().map(|arg| {
            arg.replace("{cwd}", working_directory)
               .replace("{session_id}", session_id)
        }).collect();

        // 对于 cmd/powershell，需要在工作目录下启动
        let mut cmd = Command::new(config.command);
        cmd.args(&args);

        // wezterm 通过 --cwd 参数指定目录，cmd/powershell 需要设置当前目录
        if terminal_type != "wezterm" {
            cmd.current_dir(working_directory);
        }

        cmd.creation_flags(DETACHED_PROCESS)
            .spawn()
            .map_err(|e| {
                error!("[start_terminal_with_resume] 启动失败: {}", e);
                format!("启动终端失败: {}", e)
            })?;

        info!("[start_terminal_with_resume] 终端启动成功（独立进程）");
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("[start_terminal_with_resume] 非 Windows 平台，终端配置功能受限");
        let _ = (working_directory, session_id, terminal_type);
        Err("终端恢复功能仅支持 Windows 平台".to_string())
    }

    Ok(())
}
```

---

### Task 4: Tauri 命令接口修改

**Files:**
- Modify: `src-tauri/src/commands/terminal.rs:102-118`

- [ ] **Step 1: 修改 resume_in_terminal 命令**

将函数签名和实现改为：

```rust
/// 在终端中恢复 session
#[tauri::command]
pub fn resume_in_terminal(working_directory: String, session_id: String, terminal_type: String) -> Result<(), String> {
    info!("[resume_in_terminal] 开始，工作目录: {}, session_id: {}, 终端: {}",
          working_directory, session_id, terminal_type);

    let result = start_terminal_with_resume(&working_directory, &session_id, &terminal_type);

    match result {
        Ok(_) => {
            info!("[resume_in_terminal] 完成");
            Ok(())
        }
        Err(e) => {
            error!("[resume_in_terminal] 失败: {}", e);
            Err(e)
        }
    }
}
```

---

### Task 5: 前端服务层修改

**Files:**
- Modify: `src/services/terminalService.ts:38-50`

- [ ] **Step 1: 导入 settingsStore**

在文件顶部添加导入：

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession } from '@/types'
import { useSettingsStore } from '@/stores/settingsStore'  // 新增
```

- [ ] **Step 2: 修改 resumeInTerminal 函数**

将函数改为：

```typescript
/**
 * 在终端中恢复 session
 * 启动新的终端窗口并执行 claude --resume 命令
 */
export async function resumeInTerminal(session: ClaudeSession): Promise<void> {
  const terminalType = useSettingsStore.getState().terminalType

  try {
    await invoke('resume_in_terminal', {
      workingDirectory: session.workingDirectory,
      sessionId: session.id,
      terminalType,
    })
  } catch (error) {
    // 失败时，复制恢复命令作为备用方案
    const command = `claude --resume ${session.id} --permission-mode bypassPermissions`
    await navigator.clipboard.writeText(command)
    throw new Error(`恢复失败，命令已复制到剪贴板: ${error}`)
  }
}
```

---

### Task 6: 创建设置对话框

**Files:**
- Create: `src/components/dialogs/SettingsDialog.tsx`
- Modify: `src/components/dialogs/index.ts`

- [ ] **Step 1: 创建 SettingsDialog.tsx**

```tsx
import { useState } from 'react'
import { useSettingsStore } from '@/stores/settingsStore'
import type { TerminalType } from '@/types'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Button } from '@/components/ui/button'

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

const TERMINAL_OPTIONS: { value: TerminalType; label: string }[] = [
  { value: 'wezterm', label: 'WezTerm' },
  { value: 'cmd', label: '命令提示符' },
  { value: 'powershell', label: 'PowerShell' },
]

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const { terminalType, setTerminalType } = useSettingsStore()

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>设置</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 终端选择 */}
          <div className="space-y-2">
            <label className="text-sm font-medium">默认终端</label>
            <Select
              value={terminalType}
              onValueChange={(value) => setTerminalType(value as TerminalType)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {TERMINAL_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              选择恢复 session 时使用的终端
            </p>
          </div>
        </div>

        <div className="flex justify-end">
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            关闭
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: 导出 SettingsDialog**

在 `src/components/dialogs/index.ts` 中添加导出：

```typescript
export { SettingsDialog } from './SettingsDialog'
```

---

### Task 7: 添加设置按钮到 AppLayout

**Files:**
- Modify: `src/components/layout/AppLayout.tsx:1-29`

- [ ] **Step 1: 导入 SettingsDialog 和 Settings 图标**

在文件顶部添加：

```tsx
import { useState } from 'react'
import { Settings } from 'lucide-react'
import { TabHeader } from "./TabHeader"
import { SettingsDialog } from "@/components/dialogs"
import { Button } from "@/components/ui/button"
```

- [ ] **Step 2: 添加设置按钮和对话框**

修改 AppLayout 组件：

```tsx
interface AppLayoutProps {
  children: React.ReactNode
  activeTab: string
  onTabChange: (tab: string) => void
}

const TABS = [
  { id: "running", label: "运行中" },
  { id: "management", label: "Session 管理" },
]

export function AppLayout({ children, activeTab, onTabChange }: AppLayoutProps) {
  const [settingsOpen, setSettingsOpen] = useState(false)

  return (
    <div className="flex flex-col h-screen bg-background">
      <header className="flex items-center justify-between px-4 py-2 border-b">
        <h1 className="text-lg font-semibold">Claude Fleet</h1>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSettingsOpen(true)}
          >
            <Settings className="h-4 w-4" />
          </Button>
        </div>
      </header>
      <TabHeader tabs={TABS} activeTab={activeTab} onTabChange={onTabChange} />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>

      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </div>
  )
}
```

---

### Task 8: 测试验证

- [ ] **Step 1: 运行开发模式**

```bash
npm run tauri dev
```

Expected: 应用正常启动，无编译错误

- [ ] **Step 2: 测试设置界面**

点击右上角设置图标，验证：
- 设置对话框正常打开
- 终端选择下拉框显示三个选项
- 选择后关闭对话框，再次打开时保持选择

- [ ] **Step 3: 测试终端恢复**

在 Session 管理页面选择一个历史 session：
- 点击恢复按钮
- 验证选择的终端正确启动并执行恢复命令
- 测试三种终端：wezterm、cmd、powershell

- [ ] **Step 4: 测试错误处理**

选择一个不存在的 session 或模拟终端未安装情况：
- 验证错误提示显示正确
- 命令复制到剪贴板功能正常

---

### Task 9: 提交代码

- [ ] **Step 1: Git add 和 commit**

```bash
git add src/types/settings.ts src/stores/settingsStore.ts src-tauri/src/utils/window_manager.rs src-tauri/src/commands/terminal.rs src/services/terminalService.ts src/components/dialogs/SettingsDialog.tsx src/components/dialogs/index.ts src/components/layout/AppLayout.tsx docs/superpowers/specs/2026-05-08-terminal-config-design.md docs/superpowers/plans/2026-05-08-terminal-config.md

git commit -m "feat: 终端启动命令可配置，支持 wezterm/cmd/powershell 选择"
```