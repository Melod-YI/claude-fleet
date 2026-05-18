// src/components/management/WorkspaceGroupItem.tsx

import { useState } from 'react'
import { ChevronDown, ChevronRight, Folder } from 'lucide-react'
import { SessionListItem } from './SessionListItem'

interface WorkspaceGroupItemProps {
  workspaceName: string
  sessions: Array<{
    session: any
    selected: boolean
    onToggleFavorite: () => void
    onRename?: () => void
  }>
  onSelectSession: (sessionId: string) => void
  defaultExpanded?: boolean
}

export function WorkspaceGroupItem({
  workspaceName,
  sessions,
  onSelectSession,
  defaultExpanded = true,
}: WorkspaceGroupItemProps) {
  const [expanded, setExpanded] = useState(defaultExpanded)

  return (
    <div className="border rounded-lg mb-3 overflow-hidden">
      {/* 分组头部 */}
      <div
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 px-4 py-3 bg-gray-50 hover:bg-gray-100 cursor-pointer transition-colors"
      >
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-gray-500" />
        ) : (
          <ChevronRight className="w-4 h-4 text-gray-500" />
        )}
        <Folder className="w-4 h-4 text-violet-500" />
        <span className="font-medium text-gray-700 truncate">
          {workspaceName}
        </span>
        <span className="text-sm text-gray-500 ml-auto shrink-0">
          ({sessions.length})
        </span>
      </div>

      {/* 分组内容 */}
      {expanded && (
        <div className="p-2 space-y-2 bg-white">
          {sessions.map(({ session, selected, onToggleFavorite, onRename }) => (
            <SessionListItem
              key={session.sessionId}
              session={session}
              selected={selected}
              onClick={() => onSelectSession(session.sessionId)}
              onToggleFavorite={onToggleFavorite}
              onRename={onRename}
            />
          ))}
        </div>
      )}
    </div>
  )
}