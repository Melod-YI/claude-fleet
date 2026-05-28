import { useState } from "react"
import { cn } from "@/lib/utils"
import type { SessionMeta, SessionMessage } from "@/types"
import { Button } from "@/components/ui/button"
import { ConversationView } from "./ConversationView"
import { useFavoriteStore } from "@/stores"
import { resumeInTerminal } from "@/services"
import { ConfirmDialog } from "@/components/dialogs"
import { Star, Trash2, Copy, Check, RefreshCw, Play, Clock } from "lucide-react"
import { formatRelativeTime, getDisplayName } from "@/utils"
import { PathHoverDisplay } from "@/components/common/PathHoverDisplay"
import { EditableName } from "@/components/common/EditableName"
import { setSessionName } from "@/services/dbService"

interface SessionDetailProps {
  session: SessionMeta
  messages: SessionMessage[]
  messagesLoading: boolean
  onDelete: (sessionId: string) => void
  onRefresh: () => void
}

export function SessionDetail({
  session,
  messages,
  messagesLoading,
  onDelete,
  onRefresh,
}: SessionDetailProps) {
  const displayName = getDisplayName(session)
  const [copied, setCopied] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const { toggleFavorite } = useFavoriteStore()

  const handleCopyCommand = async () => {
    const command = session.resumeCommand || `claude --resume ${session.sessionId}`
    await navigator.clipboard.writeText(command)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleResume = async () => {
    try {
      const legacySession = {
        id: session.sessionId,
        name: displayName,
        workingDirectory: session.projectDir || "",
        status: 'idle' as const,
        createdAt: session.createdAt ? new Date(session.createdAt).toISOString() : "",
        lastActivityAt: session.lastActiveAt ? new Date(session.lastActiveAt).toISOString() : "",
        conversationCount: messages.length,
        isFavorite: session.isFavorite || false,
      }
      await resumeInTerminal(legacySession)
    } catch (e) {
      alert(String(e))
    }
  }

  const handleDelete = () => {
    if (session.isFavorite) {
      alert("请先取消收藏再删除")
      return
    }
    setShowDeleteConfirm(true)
  }

  const conversationMessages = messages.map((msg) => ({
    id: `${msg.role}-${msg.ts || 0}`,
    role: msg.role as 'user' | 'assistant' | 'tool',
    content: msg.content,
    timestamp: msg.ts ? new Date(msg.ts).toISOString() : "",
  }))

  return (
    <div className="flex flex-col h-full min-w-0 overflow-hidden">
      {/* 头部信息栏 */}
      <div className="px-4 py-3 border-b bg-gray-50/50">
        {/* 标题行 */}
        <div className="flex items-center gap-3 mb-2 min-w-0">
          <EditableName
            name={displayName}
            onSave={async (newName) => {
              await setSessionName(session.sessionId, newName)
            }}
            className="text-lg font-semibold text-gray-900 min-w-0"
          />
          <div className="flex items-center gap-1.5 shrink-0 ml-auto">
            <Button
              variant="default"
              size="sm"
              onClick={handleResume}
              className="h-8 bg-violet-600 hover:bg-violet-700 shrink-0"
            >
              <Play className="w-4 h-4 mr-1" />
              恢复
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => toggleFavorite(session.sessionId)}
              className="h-8 shrink-0"
            >
              <Star className={cn(
                "w-4 h-4",
                session.isFavorite ? "fill-amber-400 text-amber-400" : "text-gray-400"
              )} />
            </Button>
            {!session.isFavorite && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleDelete}
                className="h-8 text-red-500 hover:text-red-600 hover:bg-red-50 shrink-0"
              >
                <Trash2 className="w-4 h-4" />
              </Button>
            )}
          </div>
        </div>

        {/* 元信息 */}
        <div className="flex items-center gap-3 text-sm text-gray-500 min-w-0 flex-wrap">
          {/* 路径 */}
          {session.projectDir && (
            <PathHoverDisplay path={session.projectDir} className="max-w-[400px]" />
          )}

          {/* 时间 */}
          <div className="flex items-center gap-1.5 shrink-0">
            <Clock className="w-4 h-4" />
            <span className="whitespace-nowrap">{session.lastActiveAt ? formatRelativeTime(new Date(session.lastActiveAt).toISOString()) : "未知"}</span>
          </div>

          {/* 消息数 */}
          <span className="shrink-0 whitespace-nowrap">{messages.length} 条消息</span>
        </div>

        {/* 恢复命令 */}
        <div className="mt-3 flex items-center gap-2 min-w-0">
          <code className="flex-1 min-w-0 bg-gray-100 rounded px-3 py-1.5 text-sm text-gray-600 font-mono overflow-hidden text-ellipsis whitespace-nowrap">
            {session.resumeCommand || `claude --resume ${session.sessionId}`}
          </code>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopyCommand}
            className="h-8 shrink-0"
          >
            {copied ? (
              <Check className="w-4 h-4 text-green-500" />
            ) : (
              <Copy className="w-4 h-4" />
            )}
          </Button>
        </div>
      </div>

      {/* 对话历史 */}
      <div className="flex-1 flex flex-col min-h-0 min-w-0 overflow-hidden">
        {/* 对话记录标题 */}
        <div className="flex items-center justify-between px-4 py-2 border-b bg-white min-w-0">
          <h3 className="text-sm font-medium text-gray-700 truncate">对话记录</h3>
          <Button
            variant="ghost"
            size="sm"
            onClick={onRefresh}
            className="h-7"
          >
            <RefreshCw className="w-4 h-4" />
          </Button>
        </div>

        {/* ConversationView - 添加 flex-1 min-h-0 包裹 */}
        <div className="flex-1 min-h-0 min-w-0 overflow-hidden">
          <ConversationView
            messages={conversationMessages}
            loading={messagesLoading}
          />
        </div>
      </div>

      <ConfirmDialog
        open={showDeleteConfirm}
        onClose={() => setShowDeleteConfirm(false)}
        onConfirm={() => onDelete(session.sessionId)}
        title="删除 Session"
        description={`确定要删除 "${displayName}" 吗？此操作不可撤销。`}
        confirmText="删除"
        variant="destructive"
      />
    </div>
  )
}