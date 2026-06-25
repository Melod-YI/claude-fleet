import { useMemo, useState } from "react"
import { cn } from "@/lib/utils"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { SessionCardNew } from "./SessionCard"
import { useRunningSessions } from "@/hooks/useRunningSessions"
import type { RunningSession } from "@/types"
import { useSettingsStore, useFavoriteStore } from "@/stores"
import { jumpToTerminal } from "@/services"
import { invoke } from "@tauri-apps/api/core"
import { RefreshCw, Plus } from "lucide-react"
import { Switch } from "@/components/ui/switch"
import { NewSessionDialog } from "@/components/dialogs/NewSessionDialog"

export function RunningTab() {
  const { sessions, loading, error, refresh } = useRunningSessions()
  const {
    favoritePaths,
    recordPathUsage,
    getSortedFavoritePaths
  } = useSettingsStore()
  const { toggleFavorite } = useFavoriteStore()
  const [refreshing, setRefreshing] = useState(false)
  const [searchQuery, setSearchQuery] = useState("")
  const [newSessionOpen, setNewSessionOpen] = useState(false)
  const [compact, setCompact] = useState(true) // 默认精简模式

  // 获取排序后的常用路径
  const sortedFavoritePaths = useMemo(() => {
    return getSortedFavoritePaths()
  }, [favoritePaths, getSortedFavoritePaths])

  // 搜索过滤
  const filteredSessions = useMemo(() => {
    if (searchQuery) {
      return sessions.filter((s) =>
        s.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.cwd.toLowerCase().includes(searchQuery.toLowerCase())
      )
    }
    return sessions
  }, [sessions, searchQuery])

  // 统计：idle 和 waiting 都是等待输入状态
  const waitingCount = sessions.filter((s) => s.status === "idle" || s.status === "waiting").length

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      // 派发后台全量 git 信息采集（force，绕过去重）
      await invoke('refresh_git_info_all')
    } catch (e) {
      // git 采集失败不阻塞主刷新流程
      console.warn('refresh_git_info_all 失败', e)
    }
    await refresh()
    setRefreshing(false)
  }

  const handleJumpToTerminal = async (session: RunningSession) => {
    try {
      await jumpToTerminal({
        id: session.session_id,
        workingDirectory: session.cwd,
        processId: session.pid,
        status: session.status,
        name: session.name,
      } as any)
    } catch (e) {
      alert(String(e))
    }
  }

  const handleToggleFavorite = (sessionId: string) => {
    toggleFavorite(sessionId)
  }

  const handleRecordPathUsage = (path: string) => {
    recordPathUsage(path)
  }

  const handleNewSessionClose = () => {
    setNewSessionOpen(false)
    // 刷新列表以显示新启动的 session
    refresh()
  }

  return (
    <div className="flex flex-col h-full">
      {/* 搜索栏 */}
      <div className="flex items-center gap-2 px-4 py-3 border-b bg-gray-50">
        <Input
          placeholder="搜索名称、路径..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="flex-1"
        />
        <Button
          variant="outline"
          size="icon"
          onClick={() => setNewSessionOpen(true)}
          title="新建 Session"
        >
          <Plus className="w-4 h-4" />
        </Button>
        <Button
          variant="outline"
          size="icon"
          onClick={handleRefresh}
          disabled={refreshing}
          title="刷新"
        >
          <RefreshCw className={cn("w-4 h-4", refreshing && "animate-spin")} />
        </Button>
      </div>

      {/* 状态统计 */}
      <div className="flex items-center justify-between gap-4 px-4 py-2 border-b text-sm">
        <div className="flex items-center gap-4">
          <span className="text-gray-600">
            共 {filteredSessions.length} 个运行中的 session
          </span>
          {waitingCount > 0 && (
            <span className="text-amber-600 font-medium">
              {waitingCount} 个等待输入
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-500">精简</span>
          <Switch
            checked={!compact}
            onCheckedChange={(checked) => setCompact(!checked)}
          />
          <span className="text-xs text-gray-500">详细</span>
        </div>
      </div>

      {/* Session 列表 */}
      <ScrollArea className="flex-1 p-4">
        {loading && (
          <div className="text-center text-gray-500 py-8">加载中...</div>
        )}

        {error && (
          <div className="text-center text-red-500 py-8">{error}</div>
        )}

        {!loading && !error && filteredSessions.length === 0 && (
          <div className="text-center text-gray-500 py-8">
            {searchQuery ? "没有匹配的 session" : "没有运行中的 session"}
          </div>
        )}

        {!loading && !error && filteredSessions.length > 0 && (
          <div className="flex flex-col gap-3">
            {filteredSessions.map((session) => (
              <SessionCardNew
                key={session.session_id}
                session={session}
                onJumpToTerminal={handleJumpToTerminal}
                onToggleFavorite={handleToggleFavorite}
                compact={compact}
              />
            ))}
          </div>
        )}
      </ScrollArea>

      {/* 新建 Session 弹窗 */}
      <NewSessionDialog
        open={newSessionOpen}
        onClose={handleNewSessionClose}
        favoritePaths={sortedFavoritePaths}
        onRecordPathUsage={handleRecordPathUsage}
      />
    </div>
  )
}