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