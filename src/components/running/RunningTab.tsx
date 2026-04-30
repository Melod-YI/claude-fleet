import { useMemo, useState } from "react"
import { cn } from "@/lib/utils"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { SessionCardNew } from "./SessionCard"
import { useRunningSessions, RunningSession } from "@/hooks/useRunningSessions"
import { jumpToTerminal } from "@/services"
import { RefreshCw } from "lucide-react"

export function RunningTab() {
  const { sessions, loading, error, refresh } = useRunningSessions()
  const [refreshing, setRefreshing] = useState(false)
  const [searchQuery, setSearchQuery] = useState("")

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

  // 统计
  const waitingCount = sessions.filter((s) => s.status === "waiting_input").length

  const handleRefresh = async () => {
    setRefreshing(true)
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
          onClick={handleRefresh}
          disabled={refreshing}
        >
          <RefreshCw className={cn("w-4 h-4", refreshing && "animate-spin")} />
        </Button>
      </div>

      {/* 状态统计 */}
      <div className="flex items-center gap-4 px-4 py-2 border-b text-sm">
        <span className="text-gray-600">
          共 {filteredSessions.length} 个运行中的 session
        </span>
        {waitingCount > 0 && (
          <span className="text-amber-600 font-medium">
            {waitingCount} 个等待输入
          </span>
        )}
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
              />
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  )
}