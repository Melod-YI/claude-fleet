import { useState, useEffect } from "react"
import { invoke } from '@tauri-apps/api/core'
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"
import { useNotification } from "@/hooks"
import { ErrorBoundary } from "@/components/common"

function App() {
  const [activeTab, setActiveTab] = useState("running")

  // 使用通知 hook
  useNotification()

  // 启动 sessions 目录监听服务
  useEffect(() => {
    invoke('start_sessions_watcher').catch((e) => {
      console.error('启动 sessions 监听服务失败:', e)
    })

    return () => {
      invoke('stop_sessions_watcher').catch((e) => {
        console.error('停止 sessions 监听服务失败:', e)
      })
    }
  }, [])

  return (
    <ErrorBoundary>
      <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
        {activeTab === "running" && <RunningTab />}
        {activeTab === "management" && <ManagementTab />}
      </AppLayout>
    </ErrorBoundary>
  )
}

export default App