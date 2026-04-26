# Phase 4: "Session 管理" Tab - 列表部分

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 Session 管理 Tab 左侧列表，包括搜索、收藏过滤、时间筛选、目录视图

**Architecture:** 左侧列表区域（280px 固定宽度），包含搜索栏、过滤器、session 列表项、目录视图切换

**Tech Stack:** React, TypeScript, Tailwind CSS, shadcn/ui

---

## Task 4.1: 创建 management 组件目录

**Files:**
- Create: `src/components/management/` 目录

- [ ] **Step 1: 创建目录**

```bash
mkdir -p src/components/management
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 management 组件目录"
```

---

## Task 4.2: 创建 SearchBar 组件

**Files:**
- Create: `src/components/management/SearchBar.tsx`

- [ ] **Step 1: 创建搜索栏组件**

创建 `src/components/management/SearchBar.tsx`：

```typescript
import { Input } from "@/components/ui/input"
import { Search } from "lucide-react"

interface SearchBarProps {
  value: string
  onChange: (value: string) => void
  placeholder?: string
}

export function SearchBar({ value, onChange, placeholder = "搜索..." }: SearchBarProps) {
  return (
    <div className="relative">
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="pl-9"
      />
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 SearchBar 搜索栏组件"
```

---

## Task 4.3: 创建 Toggle 开关组件

**Files:**
- Create: `src/components/common/Toggle.tsx`
- Create: `src/components/common/index.ts`

- [ ] **Step 1: 创建 common 目录**

```bash
mkdir -p src/components/common
```

- [ ] **Step 2: 创建 Toggle 组件**

创建 `src/components/common/Toggle.tsx`：

```typescript
import { cn } from "@/lib/utils"

interface ToggleProps {
  checked: boolean
  onChange: (checked: boolean) => void
  label?: string
  className?: string
}

export function Toggle({ checked, onChange, label, className }: ToggleProps) {
  return (
    <div className={cn("flex items-center gap-2", className)}>
      {label && <span className="text-sm text-gray-600">{label}</span>}
      <button
        onClick={() => onChange(!checked)}
        className={cn(
          "w-11 h-6 rounded-full relative transition-colors",
          checked ? "bg-violet-600" : "bg-gray-300"
        )}
      >
        <span
          className={cn(
            "absolute w-5 h-5 bg-white rounded-full top-0.5 transition-transform",
            checked ? "translate-x-5" : "translate-x-0.5"
          )}
        />
      </button>
    </div>
  )
}
```

- [ ] **Step 3: 创建 common 入口**

创建 `src/components/common/index.ts`：

```typescript
export { Toggle } from './Toggle'
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 创建 Toggle 开关组件"
```

---

## Task 4.4: 创建 SessionListItem 组件

**Files:**
- Create: `src/components/management/SessionListItem.tsx`

- [ ] **Step 1: 创建列表项组件**

创建 `src/components/management/SessionListItem.tsx`：

```typescript
import { cn } from "@/lib/utils"
import type { ClaudeSession } from "@/types"
import { StatusBadge } from "@/components/running"
import { formatRelativeTime } from "@/utils"
import { Star } from "lucide-react"

interface SessionListItemProps {
  session: ClaudeSession
  selected: boolean
  onClick: () => void
  onToggleFavorite: () => void
}

export function SessionListItem({ session, selected, onClick, onToggleFavorite }: SessionListItemProps) {
  const isWaitingInput = session.status === "waiting_input"

  return (
    <div
      onClick={onClick}
      className={cn(
        "p-3 rounded-md cursor-pointer transition-all",
        selected
          ? "bg-blue-100 border-l-3 border-blue-500"
          : "bg-white hover:bg-gray-50"
      )}
    >
      <div className="flex items-center justify-between mb-1">
        <span className={cn("font-medium text-sm truncate", selected ? "text-gray-900" : "text-gray-700")}>
          {session.name}
        </span>
        <button
          onClick={(e) => {
            e.stopPropagation()
            onToggleFavorite()
          }}
          className="p-0.5"
        >
          <Star
            className={cn(
              "w-4 h-4",
              session.isFavorite
                ? "fill-amber-400 text-amber-400"
                : "text-gray-300"
            )}
          />
        </button>
      </div>

      <p className="text-xs text-gray-500 truncate">{session.workingDirectory}</p>

      <div className="flex items-center gap-2 mt-2">
        <StatusBadge status={session.status} className="scale-90" />
        <span className="text-xs text-gray-500">
          {formatRelativeTime(session.lastActivityAt)}
        </span>
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 SessionListItem 列表项组件"
```

---

## Task 4.5: 创建 TimeRangeSelect 组件

**Files:**
- Create: `src/components/management/TimeRangeSelect.tsx`

- [ ] **Step 1: 创建时间范围选择组件**

创建 `src/components/management/TimeRangeSelect.tsx`：

```typescript
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import type { SessionFilter } from "@/types"

interface TimeRangeSelectProps {
  value: SessionFilter['timeRange']
  onChange: (value: SessionFilter['timeRange']) => void
}

const TIME_RANGE_OPTIONS: { value: SessionFilter['timeRange']; label: string }[] = [
  { value: '3d', label: '近 3 天' },
  { value: '7d', label: '近 7 天' },
  { value: '30d', label: '近 30 天' },
  { value: 'all', label: '全部时间' },
]

export function TimeRangeSelect({ value, onChange }: TimeRangeSelectProps) {
  return (
    <Select value={value || '30d'} onValueChange={onChange}>
      <SelectTrigger className="w-[120px] h-8">
        <SelectValue placeholder="选择时间范围" />
      </SelectTrigger>
      <SelectContent>
        {TIME_RANGE_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 TimeRangeSelect 时间范围选择组件"
```

---

## Task 4.6: 创建 DirectoryTree 组件

**Files:**
- Create: `src/components/management/DirectoryTree.tsx`

- [ ] **Step 1: 创建目录树组件**

创建 `src/components/management/DirectoryTree.tsx`：

```typescript
import { useState } from "react"
import { cn } from "@/lib/utils"
import type { ClaudeSession } from "@/types"
import { StatusBadge } from "@/components/running"
import { ChevronRight, ChevronDown, Folder, Star } from "lucide-react"

interface TreeNode {
  path: string
  name: string
  sessions: ClaudeSession[]
  children: TreeNode[]
}

interface DirectoryTreeProps {
  sessions: ClaudeSession[]
  selectedSessionId: string | null
  onSelectSession: (session: ClaudeSession) => void
  onToggleFavorite: (sessionId: string) => void
}

// 构建树结构
function buildTree(sessions: ClaudeSession[]): TreeNode[] {
  const rootMap = new Map<string, TreeNode>()

  for (const session of sessions) {
    const pathParts = session.workingDirectory.split(/[/\\]/).filter(Boolean)
    const rootPath = pathParts[0] || 'root'

    if (!rootMap.has(rootPath)) {
      rootMap.set(rootPath, {
        path: rootPath,
        name: rootPath,
        sessions: [],
        children: [],
      })
    }

    const root = rootMap.get(rootPath)!
    root.sessions.push(session)

    // 添加子路径
    if (pathParts.length > 1) {
      let current = root
      for (let i = 1; i < pathParts.length; i++) {
        const part = pathParts[i]
        let child = current.children.find((c) => c.name === part)
        if (!child) {
          child = {
            path: pathParts.slice(0, i + 1).join('/'),
            name: part,
            sessions: [],
            children: [],
          }
          current.children.push(child)
        }
        current = child
      }
      current.sessions.push(session)
    }
  }

  return Array.from(rootMap.values())
}

interface TreeNodeItemProps {
  node: TreeNode
  level: number
  expanded: boolean
  onToggleExpand: () => void
  selectedSessionId: string | null
  onSelectSession: (session: ClaudeSession) => void
  onToggleFavorite: (sessionId: string) => void
}

function TreeNodeItem({
  node,
  level,
  expanded,
  onToggleExpand,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
}: TreeNodeItemProps) {
  const hasChildren = node.children.length > 0 || node.sessions.length > 0

  return (
    <div className="select-none">
      {/* 目录节点 */}
      {hasChildren && (
        <div
          onClick={onToggleExpand}
          className={cn(
            "flex items-center gap-1 py-1 px-2 cursor-pointer hover:bg-gray-100 rounded",
            level > 0 && `ml-${level * 4}`
          )}
          style={{ marginLeft: level * 16 }}
        >
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-gray-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-gray-400" />
          )}
          <Folder className="w-4 h-4 text-violet-500" />
          <span className="font-medium text-sm">{node.name}</span>
          <span className="text-xs text-gray-400 ml-1">
            ({node.sessions.length + node.children.reduce((sum, c) => sum + c.sessions.length, 0)})
          </span>
        </div>
      )}

      {/* 展开的 session */}
      {expanded && node.sessions.map((session) => (
        <div
          key={session.id}
          onClick={() => onSelectSession(session)}
          className={cn(
            "flex items-center gap-2 py-1.5 px-2 cursor-pointer rounded",
            "ml-4",
            selectedSessionId === session.id
              ? "bg-blue-100"
              : "hover:bg-gray-50",
            session.status === "waiting_input" && "bg-amber-50"
          )}
          style={{ marginLeft: (level + 1) * 16 }}
        >
          <StatusBadge status={session.status} className="scale-75" />
          <span className="text-sm truncate">{session.name}</span>
          <button
            onClick={(e) => {
              e.stopPropagation()
              onToggleFavorite(session.id)
            }}
            className="ml-auto p-0.5"
          >
            <Star
              className={cn(
                "w-3.5 h-3.5",
                session.isFavorite
                  ? "fill-amber-400 text-amber-400"
                  : "text-gray-300"
              )}
            />
          </button>
        </div>
      ))}

      {/* 展开的子目录 */}
      {expanded && node.children.map((child) => (
        <TreeNodeItem
          key={child.path}
          node={child}
          level={level + 1}
          expanded={false}
          onToggleExpand={() => {}}
          selectedSessionId={selectedSessionId}
          onSelectSession={onSelectSession}
          onToggleFavorite={onToggleFavorite}
        />
      ))}
    </div>
  )
}

export function DirectoryTree({
  sessions,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
}: DirectoryTreeProps) {
  const tree = buildTree(sessions)
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set())

  const toggleExpand = (path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  return (
    <div className="py-2">
      {tree.map((node) => (
        <TreeNodeItem
          key={node.path}
          node={node}
          level={0}
          expanded={expandedPaths.has(node.path)}
          onToggleExpand={() => toggleExpand(node.path)}
          selectedSessionId={selectedSessionId}
          onSelectSession={onSelectSession}
          onToggleFavorite={onToggleFavorite}
        />
      ))}
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 DirectoryTree 目录树组件"
```

---

## Task 4.7: 创建 SessionList 主组件

**Files:**
- Create: `src/components/management/SessionList.tsx`

- [ ] **Step 1: 创建 SessionList 主组件**

创建 `src/components/management/SessionList.tsx`：

```typescript
import { useState, useMemo } from "react"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Button } from "@/components/ui/button"
import { SearchBar } from "./SearchBar"
import { Toggle } from "@/components/common"
import { TimeRangeSelect } from "./TimeRangeSelect"
import { SessionListItem } from "./SessionListItem"
import { DirectoryTree } from "./DirectoryTree"
import { useSessions } from "@/hooks"
import { Plus, List, FolderTree } from "lucide-react"
import type { ClaudeSession } from "@/types"

interface SessionListProps {
  selectedSessionId: string | null
  onSelectSession: (session: ClaudeSession) => void
  onNewSession: () => void
}

export function SessionList({ selectedSessionId, onSelectSession, onNewSession }: SessionListProps) {
  const { sessions, filter, setFilter, toggleFavorite } = useSessions()
  const [searchQuery, setSearchQuery] = useState("")
  const [viewMode, setViewMode] = useState<'list' | 'tree'>('list')

  // 应用搜索过滤
  const filteredSessions = useMemo(() => {
    if (!searchQuery) return sessions
    return sessions.filter((s) =>
      s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.workingDirectory.toLowerCase().includes(searchQuery.toLowerCase())
    )
  }, [sessions, searchQuery])

  const handleToggleFavoritesOnly = (checked: boolean) => {
    setFilter({ showFavoritesOnly: checked })
  }

  const handleTimeRangeChange = (value: '3d' | '7d' | '30d' | 'all') => {
    setFilter({ timeRange: value })
  }

  return (
    <div className="flex flex-col h-full bg-gray-50 border-r">
      {/* 头部 */}
      <div className="p-3 border-b bg-white">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <span className="font-semibold text-sm">Session 列表</span>
            <span className="text-xs text-gray-500">({sessions.length})</span>
          </div>
          <Button
            variant="default"
            size="sm"
            onClick={onNewSession}
            className="h-7 px-2 bg-violet-600 hover:bg-violet-700"
          >
            <Plus className="w-4 h-4" />
          </Button>
        </div>

        {/* 搜索和过滤 */}
        <div className="flex items-center gap-2">
          <SearchBar
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="搜索名称、路径、对话内容..."
          />
          <Toggle
            checked={filter.showFavoritesOnly}
            onChange={handleToggleFavoritesOnly}
            label="仅收藏"
          />
        </div>

        {/* 时间筛选（仅在显示全部时出现） */}
        {!filter.showFavoritesOnly && (
          <div className="mt-2">
            <TimeRangeSelect
              value={filter.timeRange}
              onChange={handleTimeRangeChange}
            />
          </div>
        )}
      </div>

      {/* 视图切换 */}
      <div className="flex items-center gap-1 p-2 border-b bg-gray-100">
        <Button
          variant={viewMode === 'list' ? 'default' : 'ghost'}
          size="sm"
          onClick={() => setViewMode('list')}
          className="h-7 px-2"
        >
          <List className="w-4 h-4" />
        </Button>
        <Button
          variant={viewMode === 'tree' ? 'default' : 'ghost'}
          size="sm"
          onClick={() => setViewMode('tree')}
          className="h-7 px-2"
        >
          <FolderTree className="w-4 h-4" />
        </Button>
      </div>

      {/* Session 列表 */}
      <ScrollArea className="flex-1 p-2">
        {filteredSessions.length === 0 ? (
          <div className="text-center text-gray-500 py-8 text-sm">
            {searchQuery ? "没有匹配的 session" : "没有 session"}
          </div>
        ) : viewMode === 'list' ? (
          <div className="flex flex-col gap-1">
            {filteredSessions.map((session) => (
              <SessionListItem
                key={session.id}
                session={session}
                selected={selectedSessionId === session.id}
                onClick={() => onSelectSession(session)}
                onToggleFavorite={() => toggleFavorite(session.id)}
              />
            ))}
          </div>
        ) : (
          <DirectoryTree
            sessions={filteredSessions}
            selectedSessionId={selectedSessionId}
            onSelectSession={onSelectSession}
            onToggleFavorite={toggleFavorite}
          />
        )}
      </ScrollArea>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 SessionList 主组件（搜索、过滤、视图切换）"
```

---

## Task 4.8: 创建 ManagementTab 集成框架

**Files:**
- Create: `src/components/management/ManagementTab.tsx`
- Create: `src/components/management/index.ts`

- [ ] **Step 1: 创建 ManagementTab 集成组件**

创建 `src/components/management/ManagementTab.tsx`：

```typescript
import { useState } from "react"
import { SplitPane } from "@/components/layout"
import { SessionList } from "./SessionList"
import type { ClaudeSession } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<ClaudeSession | null>(null)
  const [showNewSessionDialog, setShowNewSessionDialog] = useState(false)

  const handleNewSession = () => {
    setShowNewSessionDialog(true)
    // Phase 6 实现
  }

  return (
    <SplitPane
      left={
        <SessionList
          selectedSessionId={selectedSession?.id || null}
          onSelectSession={setSelectedSession}
          onNewSession={handleNewSession}
        />
      }
      right={
        <div className="flex items-center justify-center h-full text-gray-500">
          {selectedSession
            ? `详情: ${selectedSession.name} (Phase 5 实现)`
            : "请选择一个 session"}
        </div>
      }
      leftWidth={280}
    />
  )
}
```

- [ ] **Step 2: 创建 management 入口**

创建 `src/components/management/index.ts`：

```typescript
export { ManagementTab } from './ManagementTab'
export { SessionList } from './SessionList'
export { SessionListItem } from './SessionListItem'
export { SearchBar } from './SearchBar'
export { TimeRangeSelect } from './TimeRangeSelect'
export { DirectoryTree } from './DirectoryTree'
```

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 创建 ManagementTab 集成组件（左右分栏框架）"
```

---

## Task 4.9: 集成 ManagementTab 到 App

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: 更新 App.tsx**

编辑 `src/App.tsx`：

```typescript
import { useState } from "react"
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"

function App() {
  const [activeTab, setActiveTab] = useState("running")

  return (
    <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "running" && <RunningTab />}
      {activeTab === "management" && <ManagementTab />}
    </AppLayout>
  )
}

export default App
```

- [ ] **Step 2: 验证 Session 管理 Tab**

```bash
npm run tauri dev
```

Expected:
- 切换到 "Session 管理" Tab 正常
- 左侧列表显示 session
- 搜索功能正常
- 收藏开关正常
- 时间筛选正常
- 列表/目录视图切换正常
- 点击 session 显示占位文字

- [ ] **Step 3: Commit**

```bash
git add .
git commit -m "feat: 集成 ManagementTab 到应用"
```

---

## Phase 4 完成检查

- [ ] **验证所有功能**

检查：
- Tab 切换正常
- 左侧列表显示 session
- 搜索功能正常
- "仅显示收藏"开关正常
- 时间筛选正常（显示全部时出现）
- 列表/目录视图切换正常
- 收藏按钮可切换
- 点击 session 触发选择

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 4 Session 管理 Tab 列表部分完成"
```