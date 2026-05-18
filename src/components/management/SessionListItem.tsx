import { cn } from "@/lib/utils"
import type { SessionMeta } from "@/types"
import { Clock, Star } from "lucide-react"
import { formatRelativeTime, getDisplayName } from "@/utils"
import { EditableName } from "@/components/common/EditableName"
import { setSessionName } from "@/services/dbService"

interface SessionListItemProps {
  session: SessionMeta
  selected: boolean
  onClick: () => void
  onToggleFavorite: () => void
  onRename?: () => void
}

export function SessionListItem({ session, selected, onClick, onToggleFavorite, onRename }: SessionListItemProps) {
  const displayName = getDisplayName(session)
  const lastActive = session.lastActiveAt || session.createdAt

  return (
    <div
      onClick={onClick}
      className={cn(
        "p-3 rounded-lg cursor-pointer transition-all border min-w-0",
        selected
          ? "bg-violet-50 border-violet-200 shadow-sm"
          : "bg-white border-gray-100 hover:bg-gray-50 hover:border-gray-200"
      )}
    >
      {/* 标题和收藏 */}
      <div className="flex items-start justify-between gap-2 mb-2">
        <EditableName
          name={displayName}
          onSave={async (newName) => {
            await setSessionName(session.sessionId, newName)
            onRename?.()
          }}
          className={cn(
            "font-medium text-sm leading-snug min-w-0",
            selected ? "text-violet-900" : "text-gray-900"
          )}
        />
        <button
          onClick={(e) => {
            e.stopPropagation()
            onToggleFavorite()
          }}
          className="p-1 rounded hover:bg-gray-100 shrink-0"
        >
          <Star
            className={cn(
              "w-4 h-4",
              session.isFavorite
                ? "fill-amber-400 text-amber-400"
                : "text-gray-300 hover:text-gray-400"
            )}
          />
        </button>
      </div>

      {/* 路径 */}
      {session.projectDir && (
        <p className="text-xs text-gray-500 truncate mb-2">
          {session.projectDir}
        </p>
      )}

      {/* 时间 */}
      <div className="flex items-center gap-1.5 text-xs text-gray-400">
        <Clock className="w-3.5 h-3.5" />
        <span>
          {lastActive ? formatRelativeTime(new Date(lastActive).toISOString()) : "未知时间"}
        </span>
      </div>
    </div>
  )
}