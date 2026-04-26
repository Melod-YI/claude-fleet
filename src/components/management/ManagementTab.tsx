import { useState } from "react"
import { SplitPane } from "@/components/layout"
import { SessionList } from "./SessionList"
import type { ClaudeSession } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<ClaudeSession | null>(null)
  // Phase 6 将实现新建 session 对话框
  const [_showNewSessionDialog, setShowNewSessionDialog] = useState(false)

  const handleNewSession = () => {
    setShowNewSessionDialog(true)
    // Phase 6 实现
  }

  return (
    <SplitPane
      left={
        <SessionList
          selectedSessionId={selectedSession?.id || null}
          onSelectSession={setSelectedSession}
          onNewSession={handleNewSession}
        />
      }
      right={
        <div className="flex items-center justify-center h-full text-gray-500">
          {selectedSession
            ? `详情: ${selectedSession.name} (Phase 5 实现)`
            : "请选择一个 session"}
        </div>
      }
      leftWidth={280}
    />
  )
}