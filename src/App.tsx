import { useState, useEffect } from "react"
import { invoke } from '@tauri-apps/api/core'
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"
import { useNotification } from "@/hooks"

function App() {
  const [activeTab, setActiveTab] = useState("running")

  // 使用通知 hook
  useNotification()

  // 启动钩子服务
  useEffect(() => {
    invoke('start_hooks').catch((e) => {
      console.error('启动钩子服务失败:', e)
    })

    return () => {
      invoke('stop_hooks').catch((e) => {
        console.error('停止钩子服务失败:', e)
      })
    }
  }, [])

  return (
    <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "running" && <RunningTab />}
      {activeTab === "management" && <ManagementTab />}
    </AppLayout>
  )
}

export default App