import { ScrollArea } from "@/components/ui/scroll-area"
import { SessionListItem } from "./SessionListItem"
import { RefreshCw } from "lucide-react"
import type { SessionMeta } from "@/types"

interface SessionListProps {
  sessions: SessionMeta[]
  selectedSessionId: string | null
  onSelectSession: (session: SessionMeta) => void
  onToggleFavorite: (sessionId: string) => void
  loading?: boolean
}

export function SessionList({
  sessions,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
  loading,
}: SessionListProps) {
  return (
    <ScrollArea className="flex-1 h-full">
      <div className="p-3 pr-2 min-w-0">
        {/* 数量统计 */}
        <div className="text-xs text-gray-500 mb-3 px-1">
          {sessions.length} 个 session
        </div>

        {loading ? (
          <div className="flex items-center justify-center py-12">
            <RefreshCw className="w-5 h-5 animate-spin text-gray-400" />
          </div>
        ) : sessions.length === 0 ? (
          <div className="text-center text-gray-500 py-12 text-sm">
            没有 session
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            {sessions.map((session) => (
              <SessionListItem
                key={session.sessionId}
                session={session}
                selected={selectedSessionId === session.sessionId}
                onClick={() => onSelectSession(session)}
                onToggleFavorite={() => onToggleFavorite(session.sessionId)}
              />
            ))}
          </div>
        )}
      </div>
    </ScrollArea>
  )
}