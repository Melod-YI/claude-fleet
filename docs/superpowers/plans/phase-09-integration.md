# Phase 9: 最终集成

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 整合所有功能，完善细节，打包发布

**Architecture:** 全面测试，修复问题，优化性能，打包为桌面应用

**Tech Stack:** Tauri, Vite, Electron (可选)

---

## Task 9.1: 创建应用图标

**Files:**
- Create: `src-tauri/icons/` 目录下的图标文件

- [ ] **Step 1: 准备图标文件**

创建或下载应用图标，需要以下尺寸：
- 32x32.png
- 128x128.png
- 128x128@2x.png
- icon.icns (macOS)
- icon.ico (Windows)

```bash
mkdir -p src-tauri/icons
```

- [ ] **Step 2: 使用 Tauri 图标生成工具**

```bash
npm run tauri icon
```

如果已有源图标，可以：

```bash
npm run tauri icon ./assets/logo.png
```

Expected: 各尺寸图标生成到 `src-tauri/icons/`

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建应用图标"
```

---

## Task 9.2: 完善 Tauri 配置

**Files:**
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: 完善配置**

编辑 `src-tauri/tauri.conf.json`：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Claude Fleet",
  "version": "0.1.0",
  "identifier": "com.claude-fleet.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:5173",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "Claude Fleet",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "minWidth": 800,
        "minHeight": 600,
        "center": true,
        "visible": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "languages": ["SimpChinese", "English"]
      }
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 完善 Tauri 配置"
```

---

## Task 9.3: 创建 README 文档

**Files:**
- Create: `README.md`

- [ ] **Step 1: 创建 README**

创建 `README.md`：

```markdown
# Claude Fleet

一个管理多个 Claude Code session 的桌面应用工具。

## 功能

- **状态监控**：实时显示运行中 session 状态，等待输入时主动提示
- **快速切换**：一键跳转到对应的 Windows Terminal 窗口
- **历史管理**：收藏、搜索、恢复历史 session

## 安装

下载最新的安装包：[Releases](https://github.com/xxx/claude-fleet/releases)

## 使用

### 查看运行中的 session

打开应用，默认显示"运行中" Tab，可以看到所有正在运行的 Claude Code session。

当 session 进入"等待输入"状态时，应用会发送声音和桌面通知提醒。

### 管理历史 session

切换到"Session 管理" Tab：
- 左侧列表显示收藏和历史 session
- 支持搜索（名称、路径、对话内容）
- 支持收藏过滤和时间筛选
- 支持目录视图（按路径树状展开）

### 新建 session

点击 "+" 按钮，选择工作目录，启动新的 Claude Code。

### 恢复 session

点击"恢复"按钮，自动打开新终端窗口并执行恢复命令。

### 跳转到终端

点击"跳转到终端"按钮，自动激活对应的 Windows Terminal 窗口。

## 配置 Claude Code 钩子

为了实时接收 session 状态变化，需要配置 Claude Code 钩子。详见 [docs/hooks-setup.md](docs/hooks-setup.md)。

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

## 技术栈

- Tauri 2.0
- React 18
- TypeScript 5
- Tailwind CSS 3
- shadcn/ui
- Zustand
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "docs: 创建 README 文档"
```

---

## Task 9.4: 完善错误处理

**Files:**
- Modify: 各组件的错误处理
- Create: `src/components/common/ErrorBoundary.tsx`

- [ ] **Step 1: 创建错误边界组件**

创建 `src/components/common/ErrorBoundary.tsx`：

```typescript
import { Component, ErrorInfo, ReactNode } from "react"

interface Props {
  children: ReactNode
  fallback?: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Error caught:", error, errorInfo)
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="flex items-center justify-center h-full p-4">
          <div className="text-center">
            <h2 className="text-lg font-semibold text-red-600 mb-2">出错了</h2>
            <p className="text-sm text-gray-600 mb-4">
              {this.state.error?.message || "未知错误"}
            </p>
            <button
              onClick={() => this.setState({ hasError: false, error: null })}
              className="px-4 py-2 bg-violet-600 text-white rounded-md"
            >
              重试
            </button>
          </div>
        </div>
      )
    }

    return this.props.children
  }
}
```

- [ ] **Step 2: 更新 App.tsx 使用错误边界**

编辑 `src/App.tsx`：

```typescript
import { useState, useEffect } from "react"
import { invoke } from '@tauri-apps/api/core'
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"
import { useNotification } from "@/hooks"
import { ErrorBoundary } from "@/components/common"

function App() {
  const [activeTab, setActiveTab] = useState("running")
  useNotification()

  useEffect(() => {
    invoke('start_hooks').catch((e) => {
      console.error('启动钩子服务失败:', e)
    })

    return () => {
      invoke('stop_hooks').catch((e) => {
        console.error('停止钩子服务失败:', e)
      })
    }
  }, [])

  return (
    <ErrorBoundary>
      <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
        {activeTab === "running" && <RunningTab />}
        {activeTab === "management" && <ManagementTab />}
      </AppLayout>
    </ErrorBoundary>
  )
}

export default App
```

- [ ] **Step 3: 更新 common 入口**

编辑 `src/components/common/index.ts`：

```typescript
export { Toggle } from './Toggle'
export { ErrorBoundary } from './ErrorBoundary'
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建错误边界组件并完善错误处理"
```

---

## Task 9.5: 添加删除确认弹窗

**Files:**
- Create: `src/components/dialogs/ConfirmDialog.tsx`
- Modify: `src/components/management/SessionDetail.tsx`

- [ ] **Step 1: 创建确认弹窗组件**

创建 `src/components/dialogs/ConfirmDialog.tsx`：

```typescript
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"

interface ConfirmDialogProps {
  open: boolean
  onClose: () => void
  onConfirm: () => void
  title: string
  description: string
  confirmText?: string
  cancelText?: string
  variant?: 'default' | 'destructive'
}

export function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  description,
  confirmText = "确认",
  cancelText = "取消",
  variant = 'default',
}: ConfirmDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {cancelText}
          </Button>
          <Button
            variant={variant === 'destructive' ? 'destructive' : 'default'}
            onClick={() => {
              onConfirm()
              onClose()
            }}
          >
            {confirmText}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
```

- [ ] **Step 2: 更新 dialogs 入口**

编辑 `src/components/dialogs/index.ts`：

```typescript
export { NewSessionDialog } from './NewSessionDialog'
export { ConfirmDialog } from './ConfirmDialog'
```

- [ ] **Step 3: 更新 SessionDetail 使用确认弹窗**

编辑 `src/components/management/SessionDetail.tsx`：

```typescript
import { useState } from "react"
import { ConfirmDialog } from "@/components/dialogs"
// ... 其他导入

export function SessionDetail({ ... }: SessionDetailProps) {
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  // ... 其他状态

  const handleDelete = () => {
    if (session.isFavorite) {
      alert("请先取消收藏再删除")
      return
    }
    setShowDeleteConfirm(true)
  }

  const handleConfirmDelete = () => {
    onDelete(session.id)
  }

  return (
    <>
      <div className="flex flex-col h-full bg-white">
        {/* ... 现有内容 */}
      </div>

      <ConfirmDialog
        open={showDeleteConfirm}
        onClose={() => setShowDeleteConfirm(false)}
        onConfirm={handleConfirmDelete}
        title="删除 Session"
        description={`确定要删除 "${session.name}" 吗？此操作不可撤销。`}
        confirmText="删除"
        variant="destructive"
      />
    </>
  )
}
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建确认弹窗并集成到删除功能"
```

---

## Task 9.6: 实现删除 session 的后端命令

**Files:**
- Modify: `src-tauri/src/commands/session.rs`
- Modify: `src-tauri/src/utils/claude_data.rs`

- [ ] **Step 1: 添加删除 session 函数**

编辑 `src-tauri/src/utils/claude_data.rs`：

```rust
/// 删除 session 文件
pub fn delete_session(session_id: &str) -> Result<(), String> {
    let projects_dir = get_projects_dir();

    // 遍历找到对应 session 文件
    for entry in fs::read_dir(&projects_dir)
        .map_err(|e| format!("读取项目目录失败: {}", e))?
    {
        let project_dir = entry
            .map_err(|e| format!("读取条目失败: {}", e))?
            .path();

        let sessions_dir = project_dir.join("sessions");

        if !sessions_dir.exists() {
            continue;
        }

        let session_file = sessions_dir.join(format!("{}.jsonl", session_id));

        if session_file.exists() {
            fs::remove_file(&session_file)
                .map_err(|e| format!("删除 session 文件失败: {}", e))?;
            return Ok(())
        }
    }

    Err(format!("Session {} 不存在", session_id))
}
```

- [ ] **Step 2: 添加删除命令**

编辑 `src-tauri/src/commands/session.rs`：

```rust
use crate::utils::claude_data::{get_all_sessions, get_session_conversation, delete_session, ClaudeSession, Conversation};

#[tauri::command]
pub fn delete_session_cmd(session_id: String) -> Result<(), String> {
    delete_session(&session_id)
}
```

- [ ] **Step 3: 更新 lib.rs 注册命令**

编辑 `src-tauri/src/lib.rs`：

```rust
use commands::session::{
    list_sessions,
    get_conversation,
    refresh_sessions,
    start_new_session,
    start_hooks,
    stop_hooks,
    receive_hook_event,
    delete_session_cmd,
};

.invoke_handler(tauri::generate_handler![
    // ... 其他命令
    delete_session_cmd
])
```

- [ ] **Step 4: 创建前端删除服务**

编辑 `src/services/claudeSession.ts`：

```typescript
export async function deleteSession(sessionId: string): Promise<void> {
  try {
    await invoke('delete_session_cmd', { sessionId })
  } catch (error) {
    console.error('删除 session 失败:', error)
    throw error
  }
}
```

- [ ] **Step 5: 更新 ManagementTab 处理删除**

编辑 `src/components/management/ManagementTab.tsx`：

```typescript
import { deleteSession } from '@/services'

export function ManagementTab() {
  // ...

  const handleDelete = async (sessionId: string) => {
    try {
      await deleteSession(sessionId)
      // 刷新列表
      await refresh()
      // 清空选中
      if (selectedSession?.id === sessionId) {
        setSelectedSession(null)
      }
    } catch (e) {
      alert(`删除失败: ${e}`)
    }
  }

  // ...
}
```

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "feat: 实现删除 session 功能"
```

---

## Task 9.7: 构建和打包

**Files:**
- Modify: `package.json`

- [ ] **Step 1: 完善构建脚本**

编辑 `package.json`：

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  }
}
```

- [ ] **Step 2: 执行构建**

```bash
npm run tauri:build
```

Expected: 生成安装包到 `src-tauri/target/release/bundle/`

- [ ] **Step 3: 测试安装包**

安装并运行生成的安装包，验证所有功能正常。

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 完善构建脚本并打包"
```

---

## Task 9.8: 创建 .gitignore

**Files:**
- Create: `.gitignore`

- [ ] **Step 1: 创建 gitignore**

创建 `.gitignore`：

```gitignore
# Dependencies
node_modules/

# Build output
dist/
src-tauri/target/

# IDE
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db

# Logs
*.log

# Environment
.env
.env.local

# Tauri
.superpowers/

# Cache
.cache/
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 .gitignore"
```

---

## Phase 9 完成检查

- [ ] **最终验证**

全面测试：
- 运行中 Tab 正常
- Session 管理 Tab 正常
- 新建 Session 正常
- 恢复 Session 正常
- 跳转终端正常
- 通知正常
- 删除正常
- 收藏正常
- 搜索正常
- 时间筛选正常
- 目录视图正常

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Claude Fleet 完整实现完成"

# 创建发布标签
git tag v0.1.0
```