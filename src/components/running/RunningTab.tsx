import { useState, useMemo } from "react"
import { cn } from "@/lib/utils"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { SessionCard } from "./SessionCard"
import { useSessions } from "@/hooks"
import { searchSessions } from "@/utils"
import { RefreshCw } from "lucide-react"

export function RunningTab() {
  const { sessions, refresh, toggleFavorite, loading } = useSessions()
  const [searchQuery, setSearchQuery] = useState("")
  const [refreshing, setRefreshing] = useState(false)

  // 只显示运行中和等待输入的 session
  const activeSessions = useMemo(() => {
    const active = sessions.filter(
      (s) => s.status === "running" || s.status === "waiting_input"
    )
    if (searchQuery) {
      return searchSessions(active, searchQuery, ["name", "path"])
    }
    return active
  }, [sessions, searchQuery])

  // 按状态排序：等待输入优先
  const sortedSessions = useMemo(() => {
    return activeSessions.sort((a, b) => {
      if (a.status === "waiting_input" && b.status !== "waiting_input") return -1
      if (a.status !== "waiting_input" && b.status === "waiting_input") return 1
      return 0
    })
  }, [activeSessions])

  // 统计
  const waitingCount = activeSessions.filter((s) => s.status === "waiting_input").length

  const handleRefresh = async () => {
    setRefreshing(true)
    await refresh()
    setRefreshing(false)
  }

  const handleJumpToTerminal = async (sessionId: string) => {
    // Phase 8 实现
    console.log("Jump to terminal:", sessionId)
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
          共 {activeSessions.length} 个运行中的 session
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

        {!loading && sortedSessions.length === 0 && (
          <div className="text-center text-gray-500 py-8">
            {searchQuery ? "没有匹配的 session" : "没有运行中的 session"}
          </div>
        )}

        {!loading && sortedSessions.length > 0 && (
          <div className="flex flex-col gap-3">
            {sortedSessions.map((session) => (
              <SessionCard
                key={session.id}
                session={session}
                onJumpToTerminal={handleJumpToTerminal}
                onToggleFavorite={toggleFavorite}
              />
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  )
}