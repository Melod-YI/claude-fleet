import { useState } from "react"
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"

function App() {
  const [activeTab, setActiveTab] = useState("running")

  return (
    <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
      {activeTab === "running" && <RunningTab />}
      {activeTab === "management" && <ManagementTab />}
    </AppLayout>
  )
}

export default App