# 终端启动命令可配置设计

## 概述

将恢复 session 时使用的终端从硬编码改为可配置，支持用户选择 wezterm、cmd 或 powershell 作为默认终端。

## 背景

当前实现中，`start_terminal_with_resume` 函数硬编码使用 `wezterm` 终端。用户希望能够根据自身偏好选择不同的终端。

## 需求范围

- 仅涉及终端启动命令配置
- 单一默认终端，所有恢复操作使用同一终端
- 仅终端名称可配置，启动参数固定
- 支持：wezterm、cmd、powershell（wt 暂不支持）

## 设计方案

### 方案选择：前端枚举选择 + 后端预设参数

- 前端存储终端类型标识
- 后端根据类型匹配预设参数模板执行
- 简单直接，符合"参数固定"需求

---

## 数据结构设计

### 前端类型定义

**文件：`src/types/settings.ts`**

新增终端类型定义：

```typescript
export type TerminalType = 'wezterm' | 'cmd' | 'powershell'
```

修改 `AppSettings` 接口：

```typescript
export interface AppSettings {
  favoritePaths: FavoritePaths
  defaultTimeRange: '3d' | '7d' | '30d' | 'all'
  notificationSound: boolean
  notificationDesktop: boolean
  theme: 'light' | 'dark' | 'system'
  terminalType: TerminalType  // 新增字段
}
```

默认值：`terminalType: 'wezterm'`（保持当前默认行为）

---

### 前端设置 Store

**文件：`src/stores/settingsStore.ts`**

修改 `DEFAULT_SETTINGS`，添加默认终端类型：

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

新增 `setTerminalType` 方法：

```typescript
interface SettingsState extends AppSettings {
  // ... 现有方法
  setTerminalType: (type: TerminalType) => void
}
```

---

## 后端终端参数配置

### 终端配置结构

**文件：`src-tauri/src/utils/window_manager.rs`**

定义终端配置结构体，包含命令和参数模板：

```rust
struct TerminalConfig {
    command: &'static str,
    args: Vec<&'static str>,
}
```

预设各终端配置：

| 终端 | 命令 | 参数 |
|------|------|------|
| wezterm | `wezterm` | `start --cwd {cwd} -e claude --resume {session_id} --permission-mode bypassPermissions` |
| cmd | `cmd.exe` | `/K claude --resume {session_id} --permission-mode bypassPermissions` |
| powershell | `powershell.exe` | `-NoExit -Command claude --resume {session_id} --permission-mode bypassPermissions` |

注意：
- cmd 和 powershell 不支持 `--cwd` 参数，需在工作目录下启动
- 变量 `{cwd}` 和 `{session_id}` 在执行时替换为实际值

### 函数修改

修改 `start_terminal_with_resume` 函数签名：

```rust
pub fn start_terminal_with_resume(
    working_directory: &str,
    session_id: &str,
    terminal_type: &str  // 新增参数
) -> Result<(), String>
```

实现逻辑：

1. 根据 `terminal_type` 选择对应配置
2. 替换参数模板中的变量：
   - `{cwd}` → `working_directory`
   - `{session_id}` → `session_id`
3. 对于 cmd/powershell，先切换到工作目录再执行
4. 使用 `DETACHED_PROCESS` 标志启动独立进程

---

## Tauri 命令接口

### 命令修改

**文件：`src-tauri/src/commands/terminal.rs`**

修改 `resume_in_terminal` 命令：

```rust
#[tauri::command]
pub fn resume_in_terminal(
    working_directory: String,
    session_id: String,
    terminal_type: String  // 新增参数
) -> Result<(), String> {
    info!("[resume_in_terminal] 工作目录: {}, session_id: {}, 终端: {}",
          working_directory, session_id, terminal_type);

    start_terminal_with_resume(&working_directory, &session_id, &terminal_type)
}
```

---

## 前端实现

### 服务层修改

**文件：`src/services/terminalService.ts`**

调用时从设置 Store 获取终端类型：

```typescript
import { invoke } from '@tauri-apps/api/core'
import { useSettingsStore } from '@/stores/settingsStore'

export async function resumeSession(workingDirectory: string, sessionId: string) {
  const terminalType = useSettingsStore.getState().terminalType
  return invoke('resume_in_terminal', {
    workingDirectory,
    sessionId,
    terminalType
  })
}
```

### 设置界面

**文件：`src/components/dialogs/SettingsDialog.tsx` 或相关设置组件**

添加终端选择组件：

- 使用 RadioGroup 或 Select
- 显示选项：WezTerm、命令提示符 (cmd)、PowerShell
- 值对应：'wezterm'、'cmd'、'powershell'
- 选择后调用 `setTerminalType` 更新设置

---

## 错误处理

### 后端错误

- 终端类型无效：返回 `"不支持的终端类型: {type}"`
- 终端未安装/启动失败：返回 `"启动终端失败: {error}"`

### 前端处理

- 捕获错误并显示 Toast 或错误对话框
- 提示用户检查终端是否已安装

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/types/settings.ts` | 修改 | 新增 `TerminalType` 类型，修改 `AppSettings` |
| `src/stores/settingsStore.ts` | 修改 | 新增默认值和 `setTerminalType` 方法 |
| `src-tauri/src/utils/window_manager.rs` | 修改 | 新增终端配置结构，修改 `start_terminal_with_resume` |
| `src-tauri/src/commands/terminal.rs` | 修改 | `resume_in_terminal` 新增终端类型参数 |
| `src/services/terminalService.ts` | 修改 | 调用时传递终端类型 |
| `src/components/dialogs/SettingsDialog.tsx` | 修改 | 新增终端选择 UI |

---

## 测试要点

1. 默认使用 wezterm，恢复 session 正常
2. 切换到 cmd/powershell，恢复 session 正常并在正确目录启动
3. 设置持久化，重启应用后保持选择
4. 终端未安装时显示友好错误提示