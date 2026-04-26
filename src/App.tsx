import { useState } from "react"
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"

function App() {
  const [activeTab, setActiveTab] = useState("running")

  return (
    <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "running" && <RunningTab />}
      {activeTab === "management" && (
        <div className="flex items-center justify-center h-full text-muted-foreground">
          Session 管理（Phase 4 实现）
        </div>
      )}
    </AppLayout>
  )
}

export default App