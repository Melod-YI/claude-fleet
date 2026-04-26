# Phase 5: "Session 管理" Tab - 详情部分

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现右侧详情区域，包括名称编辑、元数据、恢复命令、对话历史

**Architecture:** 右侧详情区域（自适应宽度），独立滚动，包含元数据卡片、恢复命令、对话历史视图

**Tech Stack:** React, TypeScript, Tailwind CSS, shadcn/ui

---

## Task 5.1: 创建 ConversationView 组件

**Files:**
- Create: `src/components/management/ConversationView.tsx`

- [ ] **Step 1: 创建对话历史视图组件**

创建 `src/components/management/ConversationView.tsx`：

```typescript
import { ScrollArea } from "@/components/ui/scroll-area"
import type { ConversationMessage } from "@/types"
import { cn } from "@/lib/utils"

interface ConversationViewProps {
  messages: ConversationMessage[]
  loading?: boolean
}

export function ConversationView({ messages, loading }: ConversationViewProps) {
  if (loading) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        加载对话内容...
      </div>
    )
  }

  if (messages.length === 0) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500">
        没有对话记录
      </div>
    )
  }

  return (
    <ScrollArea className="h-full">
      <div className="flex flex-col gap-4 p-4">
        {messages.map((message) => (
          <div
            key={message.id}
            className={cn(
              "flex gap-3",
              message.role === "user" ? "flex-row" : "flex-row"
            )}
          >
            {/* 头像 */}
            <div
              className={cn(
                "w-9 h-9 rounded-full flex items-center justify-center text-white text-sm font-medium",
                message.role === "user" ? "bg-violet-600" : "bg-green-600"
              )}
            >
              {message.role === "user" ? "你" : "C"}
            </div>

            {/* 消息内容 */}
            <div className="flex-1">
              <div
                className={cn(
                  "rounded-lg p-3",
                  message.role === "user"
                    ? "bg-gray-100"
                    : "bg-green-50"
                )}
              >
                <p className="text-sm whitespace-pre-wrap">{message.content}</p>
              </div>
              <span className="text-xs text-gray-500 mt-1">
                {new Date(message.timestamp).toLocaleString("zh-CN")}
              </span>
            </div>
          </div>
        ))}
      </div>
    </ScrollArea>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 ConversationView 对话历史视图组件"
```

---

## Task 5.2: 创建 SessionDetail 组件

**Files:**
- Create: `src/components/management/SessionDetail.tsx`

- [ ] **Step 1: 创建详情组件**

创建 `src/components/management/SessionDetail.tsx`：

```typescript
import { useState } from "react"
import { cn } from "@/lib/utils"
import type { ClaudeSession, Conversation } from "@/types"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { StatusBadge } from "@/components/running"
import { ConversationView } from "./ConversationView"
import { useFavoriteStore } from "@/stores"
import { ArrowLeft, Star, Trash2, Copy, Check, RefreshCw } from "lucide-react"
import { formatRelativeTime } from "@/utils"

interface SessionDetailProps {
  session: ClaudeSession
  conversation: Conversation | null
  conversationLoading: boolean
  onBack?: () => void
  onResume: (sessionId: string) => void
  onDelete: (sessionId: string) => void
  onRefresh: () => void
}

export function SessionDetail({
  session,
  conversation,
  conversationLoading,
  onBack,
  onResume,
  onDelete,
  onRefresh,
}: SessionDetailProps) {
  const [editingName, setEditingName] = useState(session.name)
  const [savingName, setSavingName] = useState(false)
  const [copied, setCopied] = useState(false)
  const { toggleFavorite } = useFavoriteStore()

  const handleSaveName = async () => {
    if (editingName === session.name) return
    setSavingName(true)
    // Phase 6 实现：保存名称到后端
    console.log("Save name:", editingName)
    setSavingName(false)
  }

  const handleCopyCommand = async () => {
    const command = `claude --resume ${session.id}`
    await navigator.clipboard.writeText(command)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleDelete = () => {
    if (session.isFavorite) {
      alert("请先取消收藏再删除")
      return
    }
    onDelete(session.id)
  }

  return (
    <div className="flex flex-col h-full bg-white">
      {/* 头部操作栏 */}
      <div className="flex items-center justify-between px-4 py-3 border-b">
        {onBack && (
          <Button variant="ghost" size="sm" onClick={onBack}>
            <ArrowLeft className="w-4 h-4" />
          </Button>
        )}
        <div className="flex items-center gap-2 ml-auto">
          <Button
            variant="default"
            size="sm"
            onClick={() => onResume(session.id)}
            className="bg-violet-600 hover:bg-violet-700"
          >
            恢复 Session
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => toggleFavorite(session.id)}
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
          {!session.isFavorite && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleDelete}
              className="text-red-500 hover:text-red-600 hover:bg-red-50"
            >
              <Trash2 className="w-4 h-4" />
            </Button>
          )}
        </div>
      </div>

      {/* 基本信息 */}
      <div className="px-4 py-3 border-b bg-gray-50">
        {/* 名称编辑 */}
        <div className="flex items-center gap-2 mb-2">
          <Input
            value={editingName}
            onChange={(e) => setEditingName(e.target.value)}
            className="font-semibold text-lg"
          />
          {editingName !== session.name && (
            <Button
              variant="default"
              size="sm"
              onClick={handleSaveName}
              disabled={savingName}
              className="bg-violet-600"
            >
              {savingName ? "保存中..." : "保存"}
            </Button>
          )}
        </div>

        {/* 路径 */}
        <div className="flex items-center gap-2 text-sm text-gray-600 mb-1">
          <span>{session.workingDirectory}</span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => navigator.clipboard.writeText(session.workingDirectory)}
            className="p-1 h-auto"
          >
            <Copy className="w-3 h-3" />
          </Button>
        </div>

        {/* 元数据 */}
        <div className="text-xs text-gray-500">
          创建: {new Date(session.createdAt).toLocaleString("zh-CN")} ·
          上次活动: {formatRelativeTime(session.lastActivityAt)} ·
          {session.conversationCount} 轮对话
        </div>

        {/* 状态 */}
        <div className="flex items-center gap-2 mt-2">
          <StatusBadge status={session.status} />
        </div>
      </div>

      {/* 恢复命令 */}
      <div className="px-4 py-2 border-b">
        <div className="flex items-center gap-2">
          <span className="text-sm text-gray-600">恢复命令：</span>
          <div className="flex-1 flex items-center gap-2 bg-gray-100 rounded-md px-3 py-1.5">
            <code className="text-sm text-gray-800 flex-1">
              claude --resume {session.id}
            </code>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleCopyCommand}
              className="p-1 h-auto"
            >
              {copied ? (
                <Check className="w-4 h-4 text-green-500" />
              ) : (
                <Copy className="w-4 h-4" />
              )}
            </Button>
          </div>
        </div>
      </div>

      {/* 对话历史 */}
      <div className="flex-1 overflow-hidden">
        <div className="flex items-center justify-between px-4 py-2 border-b">
          <h3 className="text-sm font-medium text-violet-600">历史对话</h3>
          <Button
            variant="ghost"
            size="sm"
            onClick={onRefresh}
            className="p-1 h-auto"
          >
            <RefreshCw className="w-4 h-4" />
          </Button>
        </div>
        <ConversationView
          messages={conversation?.messages || []}
          loading={conversationLoading}
        />
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add .
git commit -m "feat: 创建 SessionDetail 详情组件"
```

---

## Task 5.3: 更新 ManagementTab 集成详情组件

**Files:**
- Modify: `src/components/management/ManagementTab.tsx`

- [ ] **Step 1: 更新 ManagementTab**

编辑 `src/components/management/ManagementTab.tsx`：

```typescript
import { useState } from "react"
import { SplitPane } from "@/components/layout"
import { SessionList } from "./SessionList"
import { SessionDetail } from "./SessionDetail"
import { useSessionStore } from "@/stores"
import type { ClaudeSession } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<ClaudeSession | null>(null)
  const { currentConversation, selectSession, loading: conversationLoading, refresh } = useSessionStore()
  const [showNewSessionDialog, setShowNewSessionDialog] = useState(false)

  const handleSelectSession = async (session: ClaudeSession) => {
    setSelectedSession(session)
    await selectSession(session.id)
  }

  const handleNewSession = () => {
    setShowNewSessionDialog(true)
    // Phase 6 实现
  }

  const handleResume = (sessionId: string) => {
    // Phase 6 实现
    console.log("Resume session:", sessionId)
  }

  const handleDelete = (sessionId: string) => {
    // Phase 6 实现
    console.log("Delete session:", sessionId)
  }

  const handleRefreshConversation = async () => {
    if (selectedSession) {
      await selectSession(selectedSession.id)
    }
  }

  return (
    <SplitPane
      left={
        <SessionList
          selectedSessionId={selectedSession?.id || null}
          onSelectSession={handleSelectSession}
          onNewSession={handleNewSession}
        />
      }
      right={
        selectedSession ? (
          <SessionDetail
            session={selectedSession}
            conversation={currentConversation}
            conversationLoading={conversationLoading}
            onResume={handleResume}
            onDelete={handleDelete}
            onRefresh={handleRefreshConversation}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-gray-500">
            请从左侧列表选择一个 session
          </div>
        )
      }
      leftWidth={280}
    />
  )
}
```

- [ ] **Step 2: 更新 management 入口导出**

编辑 `src/components/management/index.ts`：

```typescript
export { ManagementTab } from './ManagementTab'
export { SessionList } from './SessionList'
export { SessionListItem } from './SessionListItem'
export { SearchBar } from './SearchBar'
export { TimeRangeSelect } from './TimeRangeSelect'
export { DirectoryTree } from './DirectoryTree'
export { SessionDetail } from './SessionDetail'
export { ConversationView } from './ConversationView'
```

- [ ] **Step 3: 验证详情功能**

```bash
npm run tauri dev
```

Expected:
- 点击左侧 session，右侧显示详情
- 名称可编辑
- 恢复命令可复制
- 对话历史显示（如果有数据）

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "feat: 集成 SessionDetail 到 ManagementTab"
```

---

## Phase 5 完成检查

- [ ] **验证所有功能**

检查：
- 点击 session 显示详情
- 名称编辑功能
- 恢复命令复制功能
- 收藏按钮切换
- 删除按钮（未收藏时显示）
- 对话历史显示

- [ ] **Final Commit**

```bash
git add .
git commit -m "complete: Phase 5 Session 管理 Tab 详情部分完成"
```