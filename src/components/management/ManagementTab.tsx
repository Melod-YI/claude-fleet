import { useState, useMemo, useEffect } from "react"
import { cn } from "@/lib/utils"
import { SessionList } from "./SessionList"
import { SessionDetail } from "./SessionDetail"
import { GroupedSessionList } from "./GroupedSessionList"
import { NewSessionDialog } from "@/components/dialogs"
import { useSessionsQuery, useSessionMessagesQuery, useDeleteSessionMutation } from "@/lib/query"
import { useSessionSearch } from "@/hooks/useSessionSearch"
import { useFavoriteStore, useSettingsStore } from "@/stores"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Toggle } from "@/components/common"
import { TimeRangeSelect } from "./TimeRangeSelect"
import { Plus, RefreshCw, Search, ChevronRight, List, FolderTree } from "lucide-react"
import type { SessionMeta } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<SessionMeta | null>(null)
  const [showNewSessionDialog, setShowNewSessionDialog] = useState(false)
  const [searchQuery, setSearchQuery] = useState("")
  const [showFavoritesOnly, setShowFavoritesOnly] = useState(true)
  const [timeRange, setTimeRange] = useState<'3d' | '7d' | '30d' | 'all'>('30d')
  const [viewMode, setViewMode] = useState<'list' | 'grouped'>('grouped')
  const [lastScanTime, setLastScanTime] = useState<Date | null>(null)

  const { data: sessionsData, isLoading: sessionsLoading, refetch: refetchSessions, dataUpdatedAt } = useSessionsQuery()

  // 记录扫描时间
  useEffect(() => {
    if (dataUpdatedAt > 0) {
      setLastScanTime(new Date(dataUpdatedAt))
    }
  }, [dataUpdatedAt])

  const { data: messages, isLoading: messagesLoading, refetch: refetchMessages } =
    useSessionMessagesQuery(selectedSession?.sessionId)
  const deleteMutation = useDeleteSessionMutation()
  const { favorites, isFavorite, toggleFavorite } = useFavoriteStore()
  const { favoritePaths, recordPathUsage, getSortedFavoritePaths } = useSettingsStore()

  const sortedFavoritePaths = useMemo(() => getSortedFavoritePaths(), [favoritePaths])
  const sessions = sessionsData ?? []

  // Merge favorite status
  const sessionsWithFavorites = useMemo(() => {
    return sessions.map((session) => ({
      ...session,
      isFavorite: isFavorite(session.sessionId),
    }))
  }, [sessions, favorites])

  // FlexSearch
  const { search } = useSessionSearch({ sessions: sessionsWithFavorites })

  // Apply filters
  const filteredSessions = useMemo(() => {
    let result = search(searchQuery)

    if (showFavoritesOnly) {
      result = result.filter((s) => s.isFavorite)
    }

    if (!showFavoritesOnly && timeRange !== 'all') {
      const now = Date.now()
      const ranges: Record<string, number> = {
        '3d': 3 * 24 * 60 * 60 * 1000,
        '7d': 7 * 24 * 60 * 60 * 1000,
        '30d': 30 * 24 * 60 * 60 * 1000,
      }
      const cutoff = now - ranges[timeRange]
      result = result.filter((s) => {
        const ts = s.lastActiveAt || s.createdAt || 0
        return ts >= cutoff
      })
    }

    return result
  }, [search, searchQuery, showFavoritesOnly, timeRange])

  const handleSelectSession = (session: SessionMeta) => {
    setSelectedSession(session)
  }

  const handleDelete = (sessionId: string) => {
    deleteMutation.mutate(sessionId)
    if (selectedSession?.sessionId === sessionId) {
      setSelectedSession(null)
    }
  }

  const handleTimeRangeChange = (value: '3d' | '7d' | '30d' | 'all' | undefined) => {
    if (value) setTimeRange(value)
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部横条 - 搜索和操作区 */}
      <div className="flex items-center gap-3 px-4 py-3 border-b bg-white">
        {/* 标题 */}
        <h2 className="text-base font-semibold text-gray-900 shrink-0">
          Session 管理
        </h2>

        <div className="w-px h-6 bg-gray-200" />

        {/* 搜索框 */}
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
          <Input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="搜索名称、路径..."
            className="pl-10 h-9"
          />
        </div>

        {/* 过滤选项 */}
        <Toggle
          checked={showFavoritesOnly}
          onChange={setShowFavoritesOnly}
          label="仅收藏"
        />

        {!showFavoritesOnly && (
          <TimeRangeSelect
            value={timeRange}
            onChange={handleTimeRangeChange}
          />
        )}

        {/* 视图切换按钮 */}
        <div className="flex items-center gap-1">
          <Button
            variant={viewMode === 'list' ? 'default' : 'ghost'}
            size="sm"
            onClick={() => setViewMode('list')}
            className="h-9"
            title="列表视图"
          >
            <List className="w-4 h-4" />
          </Button>
          <Button
            variant={viewMode === 'grouped' ? 'default' : 'ghost'}
            size="sm"
            onClick={() => setViewMode('grouped')}
            className="h-9"
            title="分组视图"
          >
            <FolderTree className="w-4 h-4" />
          </Button>
        </div>

        {/* 扫描时间和刷新按钮 */}
        <div className="flex items-center gap-2">
          {lastScanTime && (
            <span className="text-xs text-gray-400">
              扫描于 {lastScanTime.toLocaleString('zh-CN', { hour12: false })}
            </span>
          )}
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetchSessions()}
            className="h-9"
            title="刷新扫描"
          >
            <RefreshCw className={cn("w-4 h-4", sessionsLoading && "animate-spin")} />
          </Button>
        </div>

        <Button
          variant="default"
          size="sm"
          onClick={() => setShowNewSessionDialog(true)}
          className="h-9 bg-violet-600 hover:bg-violet-700"
        >
          <Plus className="w-4 h-4 mr-1" />
          新建
        </Button>
      </div>

      {/* 下方左右布局 */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* 左侧列表 */}
        <div className="w-[320px] min-w-[320px] border-r border-gray-200 flex flex-col bg-gray-50 shadow-sm overflow-hidden">
          {viewMode === 'list' ? (
            <SessionList
              sessions={filteredSessions}
              selectedSessionId={selectedSession?.sessionId || null}
              onSelectSession={handleSelectSession}
              onToggleFavorite={toggleFavorite}
              loading={sessionsLoading}
            />
          ) : (
            <GroupedSessionList
              sessions={filteredSessions}
              selectedSessionId={selectedSession?.sessionId || null}
              onSelectSession={(sessionId) => {
                const session = filteredSessions.find(s => s.sessionId === sessionId)
                if (session) handleSelectSession(session)
              }}
              onToggleFavorite={toggleFavorite}
            />
          )}
        </div>

        {/* 右侧详情 */}
        <div className="flex-1 min-w-0 bg-white overflow-hidden">
          {selectedSession ? (
            <SessionDetail
              session={selectedSession}
              messages={messages ?? []}
              messagesLoading={messagesLoading}
              onDelete={handleDelete}
              onRefresh={() => refetchMessages()}
            />
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-gray-500">
              <ChevronRight className="w-8 h-8 text-gray-300 mb-2" />
              <p className="text-sm">请从左侧列表选择一个 session</p>
            </div>
          )}
        </div>
      </div>

      <NewSessionDialog
        open={showNewSessionDialog}
        onClose={() => setShowNewSessionDialog(false)}
        favoritePaths={sortedFavoritePaths}
        onRecordPathUsage={recordPathUsage}
      />
    </div>
  )
}