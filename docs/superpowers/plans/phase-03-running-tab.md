# Phase 3: "运行中" Tab

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现运行中 session 监控 UI，展示状态和跳转终端按钮

**Architecture:** RunningTab 主组件包含搜索栏和 SessionCard 列表，等待输入的 session 高亮显示

**Tech Stack:** React, TypeScript, Tailwind CSS, shadcn/ui

---

## Task 3.1: 创建 running 组件目录

**Files:**
- Create: `src/components/running/` 目录

- [ ] **Step 1: 创建目录**

```bash
mkdir -p src/components/running
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 running 组件目录"
```

---

## Task 3.2: 创建 StatusBadge 组件

**Files:**
- Create: `src/components/running/StatusBadge.tsx`

- [ ] **Step 1: 创建状态徽章组件**

创建 `src/components/running/StatusBadge.tsx`：

```typescript
import { cn } from "@/lib/utils"
import type { SessionStatus } from "@/types"

interface StatusBadgeProps {
  status: SessionStatus
  className?: string
}

const statusConfig: Record<SessionStatus, { label: string; className: string; icon: string }> = {
  running: {
    label: "运行中",
    className: "bg-green-500 text-white",
    icon: "●",
  },
  waiting_input: {
    label: "等待输入",
    className: "bg-amber-500 text-white",
    icon: "⏳",
  },
  completed: {
    label: "已完成",
    className: "bg-gray-500 text-white",
    icon: "✓",
  },
  idle: {
    label: "空闲",
    className: "bg-gray-300 text-gray-600",
    icon: "○",
  },
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const config = statusConfig[status]

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs font-medium",
        config.className,
        className
      )}
    >
      <span>{config.icon}</span>
      <span>{config.label}</span>
    </span>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 StatusBadge 状态徽章组件"
```

---

## Task 3.3: 创建 SessionCard 组件

**Files:**
- Create: `src/components/running/SessionCard.tsx`

- [ ] **Step 1: 创建 session 卡片组件**

创建 `src/components/running/SessionCard.tsx`：

```typescript
import { cn } from "@/lib/utils"
import type { ClaudeSession } from "@/types"
import { StatusBadge } from "./StatusBadge"
import { Button } from "@/components/ui/button"
import { formatRelativeTime } from "@/utils"
import { Star } from "lucide-react"

interface SessionCardProps {
  session: ClaudeSession
  onJumpToTerminal?: (sessionId: string) => void
  onToggleFavorite?: (sessionId: string) => void
}

export function SessionCard({ session, onJumpToTerminal, onToggleFavorite }: SessionCardProps) {
  const isWaitingInput = session.status === "waiting_input"

  return (
    <div
      className={cn(
        "rounded-lg p-4 flex justify-between items-center",
        "border transition-all",
        isWaitingInput
          ? "border-amber-400 bg-amber-50 shadow-sm"
          : "border-gray-200 bg-white hover:border-gray-300"
      )}
    >
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <h3 className="font-semibold text-gray-900 truncate">{session.name}</h3>
          <StatusBadge status={session.status} />
          {session.isFavorite && (
            <Star className="w-4 h-4 fill-amber-400 text-amber-400" />
          )}
        </div>
        <p className="text-sm text-gray-600 truncate">{session.workingDirectory}</p>
        <p className="text-xs text-gray-500 mt-1">
          上次活动: {formatRelativeTime(session.lastActivityAt)}
        </p>
      </div>

      <div className="flex items-center gap-2 ml-4">
        {session.status !== "completed" && (
          <Button
            variant={isWaitingInput ? "default" : "secondary"}
            size="sm"
            onClick={() => onJumpToTerminal?.(session.id)}
            className={isWaitingInput ? "bg-violet-600 hover:bg-violet-700" : ""}
          >
            跳转到终端
          </Button>
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onToggleFavorite?.(session.id)}
          className="p-1"
        >
          <Star
            className={cn(
              "w-4 h-4",
              session.isFavorite
                ? "fill-amber-400 text-amber-400"
                : "text-gray-400"
            )}
          />
        </Button>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 SessionCard 组件（等待输入高亮）"
```

---

## Task 3.4: 创建 RunningTab 主组件

**Files:**
- Create: `src/components/running/RunningTab.tsx`
- Create: `src/components/running/index.ts`

- [ ] **Step 1: 创建 RunningTab 主组件**

创建 `src/components/running/RunningTab.tsx`：

```typescript
import { useState, useMemo } from "react"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { SessionCard } from "./SessionCard"
import { useSessions } from "@/hooks"
import { searchSessions } from "@/utils"
import { RefreshCw } from "lucide-react"

export function RunningTab() {
  const { sessions, refresh, toggleFavorite, loading } = useSessions()
  const [searchQuery, setSearchQuery] = useState("")
  const [refreshing, setRefreshing] = useState(false)

  // 只显示运行中和等待输入的 session
  const activeSessions = useMemo(() => {
    const active = sessions.filter(
      (s) => s.status === "running" || s.status === "waiting_input"
    )
    if (searchQuery) {
      return searchSessions(active, searchQuery, ["name", "path"])
    }
    return active
  }, [sessions, searchQuery])

  // 按状态排序：等待输入优先
  const sortedSessions = useMemo(() => {
    return activeSessions.sort((a, b) => {
      if (a.status === "waiting_input" && b.status !== "waiting_input") return -1
      if (a.status !== "waiting_input" && b.status === "waiting_input") return 1
      return 0
    })
  }, [activeSessions])

  // 统计
  const waitingCount = activeSessions.filter((s) => s.status === "waiting_input").length
  const runningCount = activeSessions.filter((s) => s.status === "running").length

  const handleRefresh = async () => {
    setRefreshing(true)
    await refresh()
    setRefreshing(false)
  }

  const handleJumpToTerminal = async (sessionId: string) => {
    // Phase 8 实现
    console.log("Jump to terminal:", sessionId)
  }

  return (
    <div className="flex flex-col h-full">
      {/* 搜索栏 */}
      <div className="flex items-center gap-2 px-4 py-3 border-b bg-gray-50">
        <Input
          placeholder="搜索名称、路径..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="flex-1"
        />
        <Button
          variant="outline"
          size="icon"
          onClick={handleRefresh}
          disabled={refreshing}
        >
          <RefreshCw className={cn("w-4 h-4", refreshing && "animate-spin")} />
        </Button>
      </div>

      {/* 状态统计 */}
      <div className="flex items-center gap-4 px-4 py-2 border-b text-sm">
        <span className="text-gray-600">
          共 {activeSessions.length} 个运行中的 session
        </span>
        {waitingCount > 0 && (
          <span className="text-amber-600 font-medium">
            {waitingCount} 个等待输入
          </span>
        )}
      </div>

      {/* Session 列表 */}
      <ScrollArea className="flex-1 p-4">
        {loading && (
          <div className="text-center text-gray-500 py-8">加载中...</div>
        )}

        {!loading && sortedSessions.length === 0 && (
          <div className="text-center text-gray-500 py-8">
            {searchQuery ? "没有匹配的 session" : "没有运行中的 session"}
          </div>
        )}

        {!loading && sortedSessions.length > 0 && (
          <div className="flex flex-col gap-3">
            {sortedSessions.map((session) => (
              <SessionCard
                key={session.id}
                session={session}
                onJumpToTerminal={handleJumpToTerminal}
                onToggleFavorite={toggleFavorite}
              />
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  )
}
```

- [ ] **Step 2: 创建 running 入口**

创建 `src/components/running/index.ts`：

```typescript
export { RunningTab } from './RunningTab'
export { SessionCard } from './SessionCard'
export { StatusBadge } from './StatusBadge'
```

- [ ] **Step 3: 添加缺失的导入**

编辑 `src/components/running/RunningTab.tsx`，添加 cn 导入：

```typescript
import { cn } from "@/lib/utils"
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建 RunningTab 主组件（搜索、排序、刷新）"
```

---

## Task 3.5: 集成 RunningTab 到 App

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/layout/AppLayout.tsx`

- [ ] **Step 1: 更新 AppLayout 支持动态内容**

编辑 `src/components/layout/AppLayout.tsx`：

```typescript
import { useState } from "react"
import { TabHeader } from "./TabHeader"

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
  return (
    <div className="flex flex-col h-screen bg-background">
      <header className="flex items-center justify-between px-4 py-2 border-b">
        <h1 className="text-lg font-semibold">Claude Fleet</h1>
        <div className="flex items-center gap-2">
          {/* 后续添加设置按钮 */}
        </div>
      </header>
      <TabHeader tabs={TABS} activeTab={activeTab} onTabChange={onTabChange} />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>
    </div>
  )
}
```

- [ ] **Step 2: 更新 App.tsx 集成 RunningTab**

编辑 `src/App.tsx`：

```typescript
import { useState } from "react"
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"

function App() {
  const [activeTab, setActiveTab] = useState("running")

  return (
    <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "running" && <RunningTab />}
      {activeTab === "management" && (
        <div className="flex items-center justify-center h-full text-muted-foreground">
          Session 管理（Phase 4 实现）
        </div>
      )}
    </AppLayout>
  )
}

export default App
```

- [ ] **Step 3: 验证 RunningTab**

```bash
npm run tauri dev
```

Expected:
- 应用显示"运行中" Tab
- 搜索栏正常
- Session 卡片列表显示（如果有数据）
- 等待输入的 session 高亮显示

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 集成 RunningTab 到应用"
```

---

## Phase 3 完成检查

- [ ] **验证所有功能**

检查：
- Tab 切换正常
- 搜索功能正常
- 状态徽章显示正确
- 等待输入的 session 高亮
- 收藏按钮可点击
- 刷新按钮正常

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 3 运行中 Tab 完成"
```