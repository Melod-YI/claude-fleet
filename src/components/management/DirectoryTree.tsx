import { useState } from "react"
import { cn } from "@/lib/utils"
import type { SessionMeta } from "@/types"
import { ChevronRight, ChevronDown, Folder, Star } from "lucide-react"

interface TreeNode {
  path: string
  name: string
  sessions: SessionMeta[]
  children: TreeNode[]
}

interface DirectoryTreeProps {
  sessions: SessionMeta[]
  selectedSessionId: string | null
  onSelectSession: (session: SessionMeta) => void
  onToggleFavorite: (sessionId: string) => void
}

// 构建树结构
function buildTree(sessions: SessionMeta[]): TreeNode[] {
  const rootMap = new Map<string, TreeNode>()

  for (const session of sessions) {
    const path = session.projectDir || ""
    const pathParts = path.split(/[/\\]/).filter(Boolean)
    const rootPath = pathParts[0] || 'root'

    if (!rootMap.has(rootPath)) {
      rootMap.set(rootPath, {
        path: rootPath,
        name: rootPath,
        sessions: [],
        children: [],
      })
    }

    const root = rootMap.get(rootPath)!
    root.sessions.push(session)

    // 添加子路径
    if (pathParts.length > 1) {
      let current = root
      for (let i = 1; i < pathParts.length; i++) {
        const part = pathParts[i]
        let child = current.children.find((c) => c.name === part)
        if (!child) {
          child = {
            path: pathParts.slice(0, i + 1).join('/'),
            name: part,
            sessions: [],
            children: [],
          }
          current.children.push(child)
        }
        current = child
      }
      current.sessions.push(session)
    }
  }

  return Array.from(rootMap.values())
}

interface TreeNodeItemProps {
  node: TreeNode
  level: number
  expanded: boolean
  onToggleExpand: () => void
  selectedSessionId: string | null
  onSelectSession: (session: SessionMeta) => void
  onToggleFavorite: (sessionId: string) => void
}

function TreeNodeItem({
  node,
  level,
  expanded,
  onToggleExpand,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
}: TreeNodeItemProps) {
  const hasChildren = node.children.length > 0 || node.sessions.length > 0

  return (
    <div className="select-none">
      {/* 目录节点 */}
      {hasChildren && (
        <div
          onClick={onToggleExpand}
          className={cn(
            "flex items-center gap-1 py-1 px-2 cursor-pointer hover:bg-gray-100 rounded",
            level > 0 && `ml-${level * 4}`
          )}
          style={{ marginLeft: level * 16 }}
        >
          {expanded ? (
            <ChevronDown className="w-4 h-4 text-gray-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-gray-400" />
          )}
          <Folder className="w-4 h-4 text-violet-500" />
          <span className="font-medium text-sm">{node.name}</span>
          <span className="text-xs text-gray-400 ml-1">
            ({node.sessions.length + node.children.reduce((sum, c) => sum + c.sessions.length, 0)})
          </span>
        </div>
      )}

      {/* 展开的 session */}
      {expanded && node.sessions.map((session) => {
        const title = session.title || session.projectDir?.split(/[\\/]/).pop() || session.sessionId
        return (
          <div
            key={session.sessionId}
            onClick={() => onSelectSession(session)}
            className={cn(
              "flex items-center gap-2 py-1.5 px-2 cursor-pointer rounded",
              "ml-4",
              selectedSessionId === session.sessionId
                ? "bg-blue-100"
                : "hover:bg-gray-50"
            )}
            style={{ marginLeft: (level + 1) * 16 }}
          >
            <span className="text-sm truncate">{title}</span>
            <button
              onClick={(e) => {
                e.stopPropagation()
                onToggleFavorite(session.sessionId)
              }}
              className="ml-auto p-0.5"
            >
              <Star
                className={cn(
                  "w-3.5 h-3.5",
                  session.isFavorite
                    ? "fill-amber-400 text-amber-400"
                    : "text-gray-300"
                )}
              />
            </button>
          </div>
        )
      })}

      {/* 展开的子目录 */}
      {expanded && node.children.map((child) => (
        <TreeNodeItem
          key={child.path}
          node={child}
          level={level + 1}
          expanded={false}
          onToggleExpand={() => {}}
          selectedSessionId={selectedSessionId}
          onSelectSession={onSelectSession}
          onToggleFavorite={onToggleFavorite}
        />
      ))}
    </div>
  )
}

export function DirectoryTree({
  sessions,
  selectedSessionId,
  onSelectSession,
  onToggleFavorite,
}: DirectoryTreeProps) {
  const tree = buildTree(sessions)
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set())

  const toggleExpand = (path: string) => {
    setExpandedPaths((prev) => {
      const next = new Set(prev)
      if (next.has(path)) {
        next.delete(path)
      } else {
        next.add(path)
      }
      return next
    })
  }

  return (
    <div className="py-2">
      {tree.map((node) => (
        <TreeNodeItem
          key={node.path}
          node={node}
          level={0}
          expanded={expandedPaths.has(node.path)}
          onToggleExpand={() => toggleExpand(node.path)}
          selectedSessionId={selectedSessionId}
          onSelectSession={onSelectSession}
          onToggleFavorite={onToggleFavorite}
        />
      ))}
    </div>
  )
}