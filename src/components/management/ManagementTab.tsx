import { useState } from "react"
import { SplitPane } from "@/components/layout"
import { SessionList } from "./SessionList"
import { SessionDetail } from "./SessionDetail"
import { useSessionStore } from "@/stores"
import type { ClaudeSession } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<ClaudeSession | null>(null)
  const { currentConversation, selectSession, loading: conversationLoading } = useSessionStore()
  const [_showNewSessionDialog, setShowNewSessionDialog] = useState(false)

  const handleSelectSession = async (session: ClaudeSession) => {
    setSelectedSession(session)
    await selectSession(session.id)
  }

  const handleNewSession = () => {
    setShowNewSessionDialog(true)
    // Phase 6 实现
  }

  const handleResume = (sessionId: string) => {
    // Phase 6 实现
    console.log("Resume session:", sessionId)
  }

  const handleDelete = (sessionId: string) => {
    // Phase 6 实现
    console.log("Delete session:", sessionId)
  }

  const handleRefreshConversation = async () => {
    if (selectedSession) {
      await selectSession(selectedSession.id)
    }
  }

  return (
    <SplitPane
      left={
        <SessionList
          selectedSessionId={selectedSession?.id || null}
          onSelectSession={handleSelectSession}
          onNewSession={handleNewSession}
        />
      }
      right={
        selectedSession ? (
          <SessionDetail
            session={selectedSession}
            conversation={currentConversation}
            conversationLoading={conversationLoading}
            onResume={handleResume}
            onDelete={handleDelete}
            onRefresh={handleRefreshConversation}
          />
        ) : (
          <div className="flex items-center justify-center h-full text-gray-500">
            请从左侧列表选择一个 session
          </div>
        )
      }
      leftWidth={280}
    />
  )
}