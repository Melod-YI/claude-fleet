import { cn } from "@/lib/utils"
import type { ClaudeSession } from "@/types"
import { StatusBadge } from "./StatusBadge"
import { Button } from "@/components/ui/button"
import { formatRelativeTime, formatRelativeTimeFromTimestamp } from "@/utils"
import { jumpToTerminal } from "@/services"
import { Star } from "lucide-react"
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
}

export function SessionCardNew({ session, onJumpToTerminal }: SessionCardNewProps) {
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
        <p className="text-sm text-gray-600 truncate">{session.cwd}</p>
        <p className="text-xs text-gray-500 mt-1">
          上次活动: {formatRelativeTimeFromTimestamp(session.updated_at)} · PID: {session.pid}
        </p>
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