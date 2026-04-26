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