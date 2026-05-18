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

/**
 * 从项目目录路径提取最后一层文件夹名作为显示名
 * 例如: C:\workspace\claude-fleet-sp -> claude-fleet-sp
 */
function extractDisplayName(projectDir?: string): string {
  if (!projectDir) return '未知项目'

  const parts = projectDir.split(/[\\/]/).filter(Boolean)
  return parts.pop() || '未知项目'
}

export function GroupedSessionList({
  sessions,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
  onRename,
}: GroupedSessionListProps) {
  const grouped = useMemo(() => {
    const groups: Map<string, { displayName: string; sessions: SessionMeta[] }> = new Map()

    for (const session of sessions) {
      // 使用 projectDir 作为分组依据
      const workspacePath = session.projectDir || ''
      if (!groups.has(workspacePath)) {
        groups.set(workspacePath, {
          displayName: extractDisplayName(workspacePath),
          sessions: [],
        })
      }
      groups.get(workspacePath)!.sessions.push(session)
    }

    return Array.from(groups.entries())
      .sort((a, b) => a[1].displayName.localeCompare(b[1].displayName))
      .map(([workspacePath, { displayName, sessions: sessionList }]) => ({
        workspacePath,
        workspaceName: displayName,
        sessions: sessionList.sort((a, b) =>
          (b.lastActiveAt || 0) - (a.lastActiveAt || 0)
        ),
      }))
  }, [sessions])

  return (
    <div className="flex-1 overflow-y-auto p-3 space-y-3">
      {grouped.map(({ workspaceName, workspacePath, sessions: groupSessions }) => (
        <WorkspaceGroupItem
          key={workspacePath || workspaceName}
          workspaceName={workspaceName}
          workspacePath={workspacePath}
          sessions={groupSessions.map(session => ({
            session,
            selected: selectedSessionId === session.sessionId,
            onToggleFavorite: () => onToggleFavorite(session.sessionId),
            onRename: onRename ? () => onRename(session.sessionId) : undefined,
          }))}
          onSelectSession={onSelectSession}
          defaultExpanded={false}
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