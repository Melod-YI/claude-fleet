// src/components/management/GroupedSessionList.tsx

import { useMemo } from 'react'
import type { SessionMeta } from '@/types'
import { WorkspaceGroupItem } from './WorkspaceGroupItem'

interface GroupedSessionListProps {
  sessions: SessionMeta[]
  selectedSessionId: string | null
  onSelectSession: (sessionId: string) => void
  onToggleFavorite: (sessionId: string) => void
  onRename?: (sessionId: string) => void
}

function extractWorkspaceName(sourcePath?: string): string {
  if (!sourcePath) return '未知项目'

  const parts = sourcePath.split(/[\\/]/)
  const projectsIndex = parts.findIndex(p => p === 'projects')
  if (projectsIndex >= 0 && parts.length > projectsIndex + 1) {
    return parts[projectsIndex + 1]
  }

  return '未知项目'
}

export function GroupedSessionList({
  sessions,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
  onRename,
}: GroupedSessionListProps) {
  const grouped = useMemo(() => {
    const groups: Map<string, SessionMeta[]> = new Map()

    for (const session of sessions) {
      const workspace = extractWorkspaceName(session.sourcePath)
      if (!groups.has(workspace)) {
        groups.set(workspace, [])
      }
      groups.get(workspace)!.push(session)
    }

    return Array.from(groups.entries())
      .sort((a, b) => a[0].localeCompare(b[0]))
      .map(([workspaceName, sessionList]) => ({
        workspaceName,
        sessions: sessionList.sort((a, b) =>
          (b.lastActiveAt || 0) - (a.lastActiveAt || 0)
        ),
      }))
  }, [sessions])

  return (
    <div className="space-y-3">
      {grouped.map(({ workspaceName, sessions: groupSessions }) => (
        <WorkspaceGroupItem
          key={workspaceName}
          workspaceName={workspaceName}
          sessions={groupSessions.map(session => ({
            session,
            selected: selectedSessionId === session.sessionId,
            onToggleFavorite: () => onToggleFavorite(session.sessionId),
            onRename: onRename ? () => onRename(session.sessionId) : undefined,
          }))}
          onSelectSession={onSelectSession}
          defaultExpanded={true}
        />
      ))}

      {grouped.length === 0 && (
        <div className="text-center py-8 text-gray-500">
          没有收藏的 session
        </div>
      )}
    </div>
  )
}