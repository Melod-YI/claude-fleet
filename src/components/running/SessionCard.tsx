import { cn } from "@/lib/utils"
import type { ClaudeSession } from "@/types"
import { StatusBadge } from "./StatusBadge"
import { Button } from "@/components/ui/button"
import { formatRelativeTime, formatRelativeTimeFromTimestamp } from "@/utils"
import { jumpToTerminal } from "@/services"
import { Star, Folder, Clock } from "lucide-react"
import type { RunningSession } from "@/hooks/useRunningSessions"

interface SessionCardProps {
  session: ClaudeSession
  onJumpToTerminal?: (sessionId: string) => void
  onToggleFavorite?: (sessionId: string) => void
}

export function SessionCard({ session, onJumpToTerminal, onToggleFavorite }: SessionCardProps) {
  // idle 和 waiting 都是等待输入状态
  const isWaitingInput = session.status === "idle" || session.status === "waiting"

  const handleJump = async () => {
    try {
      await jumpToTerminal(session)
    } catch (e) {
      // 调用备用方案或显示错误
      if (onJumpToTerminal) {
        onJumpToTerminal(session.id)
      } else {
        alert(String(e))
      }
    }
  }

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
        <Button
          variant={isWaitingInput ? "default" : "secondary"}
          size="sm"
          onClick={handleJump}
          className={isWaitingInput ? "bg-violet-600 hover:bg-violet-700" : ""}
        >
          跳转到终端
        </Button>
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

interface SessionCardNewProps {
  session: RunningSession
  onJumpToTerminal: (session: RunningSession) => void
  compact?: boolean // 精简模式，默认 true
}

export function SessionCardNew({ session, onJumpToTerminal, compact = true }: SessionCardNewProps) {
  // idle 和 waiting 都是等待输入状态
  const isWaitingInput = session.status === "idle" || session.status === "waiting"

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
        </div>

        {/* 最后用户输入 - 单行卡片（详细模式下显示） */}
        {!compact && session.last_user_input && (
          <div className="mt-2 px-2 py-1 bg-blue-50 rounded-md border border-blue-100 flex items-center gap-2">
            <span className="text-xs font-medium text-blue-600 shrink-0">最后输入:</span>
            <span
              className="text-sm text-gray-700 truncate"
              title={session.last_user_input}
            >
              {session.last_user_input}
            </span>
          </div>
        )}

        {/* away_summary 展示 - 单行卡片（详细模式下显示） */}
        {!compact && isWaitingInput && session.away_summary && (
          <div className="mt-2 px-2 py-1 bg-violet-50 rounded-md border border-violet-100 flex items-center gap-2">
            <span className="text-xs font-medium text-violet-600 shrink-0">最近进展:</span>
            {session.away_summary_at && (
              <span className="text-xs text-violet-400 shrink-0">
                {formatRelativeTimeFromTimestamp(session.away_summary_at)}
              </span>
            )}
            <span
              className="text-sm text-gray-700 truncate"
              title={session.away_summary}
            >
              {session.away_summary}
            </span>
          </div>
        )}

        {/* 元信息行 */}
        <div className="text-xs text-gray-500 mt-2 flex flex-wrap items-center gap-x-2">
          <span
            className="flex items-center gap-1 truncate max-w-[200px]"
            title={session.cwd}
          >
            <Folder className="w-3 h-3 text-gray-400 shrink-0" />
            {session.cwd.split(/[\\/]/).filter(Boolean).pop() || session.cwd}
          </span>
          <span className="text-gray-300">|</span>
          <span className="flex items-center gap-1">
            <Clock className="w-3 h-3 text-gray-400" />
            {formatRelativeTimeFromTimestamp(session.updated_at)}
          </span>
          <span className="text-gray-300">|</span>
          <span>PID: {session.pid}</span>
          <span className="text-gray-300">|</span>
          <span>Session ID: {session.session_id}</span>
        </div>
      </div>

      <div className="flex items-center gap-2 ml-4">
        <Button
          variant={isWaitingInput ? "default" : "secondary"}
          size="sm"
          onClick={() => onJumpToTerminal(session)}
          className={isWaitingInput ? "bg-violet-600 hover:bg-violet-700" : ""}
        >
          跳转到终端
        </Button>
      </div>
    </div>
  )
}