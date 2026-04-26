import { useState, useMemo } from "react"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Button } from "@/components/ui/button"
import { SearchBar } from "./SearchBar"
import { Toggle } from "@/components/common"
import { TimeRangeSelect } from "./TimeRangeSelect"
import { SessionListItem } from "./SessionListItem"
import { DirectoryTree } from "./DirectoryTree"
import { useSessions } from "@/hooks"
import { Plus, List, FolderTree } from "lucide-react"
import type { ClaudeSession } from "@/types"

interface SessionListProps {
  selectedSessionId: string | null
  onSelectSession: (session: ClaudeSession) => void
  onNewSession: () => void
}

export function SessionList({ selectedSessionId, onSelectSession, onNewSession }: SessionListProps) {
  const { sessions, filter, setFilter, toggleFavorite } = useSessions()
  const [searchQuery, setSearchQuery] = useState("")
  const [viewMode, setViewMode] = useState<'list' | 'tree'>('list')

  // 应用搜索过滤
  const filteredSessions = useMemo(() => {
    if (!searchQuery) return sessions
    return sessions.filter((s) =>
      s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      s.workingDirectory.toLowerCase().includes(searchQuery.toLowerCase())
    )
  }, [sessions, searchQuery])

  const handleToggleFavoritesOnly = (checked: boolean) => {
    setFilter({ showFavoritesOnly: checked })
  }

  const handleTimeRangeChange = (value: '3d' | '7d' | '30d' | 'all' | undefined) => {
    setFilter({ timeRange: value })
  }

  return (
    <div className="flex flex-col h-full bg-gray-50 border-r">
      {/* 头部 */}
      <div className="p-3 border-b bg-white">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <span className="font-semibold text-sm">Session 列表</span>
            <span className="text-xs text-gray-500">({sessions.length})</span>
          </div>
          <Button
            variant="default"
            size="sm"
            onClick={onNewSession}
            className="h-7 px-2 bg-violet-600 hover:bg-violet-700"
          >
            <Plus className="w-4 h-4" />
          </Button>
        </div>

        {/* 搜索和过滤 */}
        <div className="flex items-center gap-2">
          <SearchBar
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="搜索名称、路径、对话内容..."
          />
          <Toggle
            checked={filter.showFavoritesOnly}
            onChange={handleToggleFavoritesOnly}
            label="仅收藏"
          />
        </div>

        {/* 时间筛选（仅在显示全部时出现） */}
        {!filter.showFavoritesOnly && (
          <div className="mt-2">
            <TimeRangeSelect
              value={filter.timeRange}
              onChange={handleTimeRangeChange}
            />
          </div>
        )}
      </div>

      {/* 视图切换 */}
      <div className="flex items-center gap-1 p-2 border-b bg-gray-100">
        <Button
          variant={viewMode === 'list' ? 'default' : 'ghost'}
          size="sm"
          onClick={() => setViewMode('list')}
          className="h-7 px-2"
        >
          <List className="w-4 h-4" />
        </Button>
        <Button
          variant={viewMode === 'tree' ? 'default' : 'ghost'}
          size="sm"
          onClick={() => setViewMode('tree')}
          className="h-7 px-2"
        >
          <FolderTree className="w-4 h-4" />
        </Button>
      </div>

      {/* Session 列表 */}
      <ScrollArea className="flex-1 p-2">
        {filteredSessions.length === 0 ? (
          <div className="text-center text-gray-500 py-8 text-sm">
            {searchQuery ? "没有匹配的 session" : "没有 session"}
          </div>
        ) : viewMode === 'list' ? (
          <div className="flex flex-col gap-1">
            {filteredSessions.map((session) => (
              <SessionListItem
                key={session.id}
                session={session}
                selected={selectedSessionId === session.id}
                onClick={() => onSelectSession(session)}
                onToggleFavorite={() => toggleFavorite(session.id)}
              />
            ))}
          </div>
        ) : (
          <DirectoryTree
            sessions={filteredSessions}
            selectedSessionId={selectedSessionId}
            onSelectSession={onSelectSession}
            onToggleFavorite={toggleFavorite}
          />
        )}
      </ScrollArea>
    </div>
  )
}