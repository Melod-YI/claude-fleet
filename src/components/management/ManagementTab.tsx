import { useState } from "react"
import { SplitPane } from "@/components/layout"
import { SessionList } from "./SessionList"
import { SessionDetail } from "./SessionDetail"
import { NewSessionDialog } from "@/components/dialogs"
import { useSessionStore, useSettingsStore } from "@/stores"
import { deleteSession } from "@/services"
import type { ClaudeSession } from "@/types"

export function ManagementTab() {
  const [selectedSession, setSelectedSession] = useState<ClaudeSession | null>(null)
  const { currentConversation, selectSession, loading: conversationLoading } = useSessionStore()
  const { favoritePaths, addFavoritePath } = useSettingsStore()
  const [showNewSessionDialog, setShowNewSessionDialog] = useState(false)

  const handleSelectSession = async (session: ClaudeSession) => {
    setSelectedSession(session)
    await selectSession(session.id)
  }

  const handleNewSession = () => {
    setShowNewSessionDialog(true)
  }

  const handleCloseNewSessionDialog = () => {
    setShowNewSessionDialog(false)
  }

  const handleDelete = async (sessionId: string) => {
    try {
      await deleteSession(sessionId)
      // 清空选中
      if (selectedSession?.id === sessionId) {
        setSelectedSession(null)
      }
      // 通知 SessionList 刷新（通过 useSessionStore）
      await selectSession("")
    } catch (e) {
      alert(`删除失败: ${e}`)
    }
  }

  const handleRefreshConversation = async () => {
    if (selectedSession) {
      await selectSession(selectedSession.id)
    }
  }

  return (
    <>
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

      <NewSessionDialog
        open={showNewSessionDialog}
        onClose={handleCloseNewSessionDialog}
        favoritePaths={favoritePaths.paths}
        onAddFavoritePath={addFavoritePath}
      />
    </>
  )
}