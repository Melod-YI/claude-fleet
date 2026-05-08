import { useState } from 'react'
import { Settings } from 'lucide-react'
import { TabHeader } from "./TabHeader"
import { SettingsDialog } from "@/components/dialogs"
import { Button } from "@/components/ui/button"

interface AppLayoutProps {
  children: React.ReactNode
  activeTab: string
  onTabChange: (tab: string) => void
}

const TABS = [
  { id: "running", label: "运行中" },
  { id: "management", label: "Session 管理" },
]

export function AppLayout({ children, activeTab, onTabChange }: AppLayoutProps) {
  const [settingsOpen, setSettingsOpen] = useState(false)

  return (
    <div className="flex flex-col h-screen bg-background">
      <header className="flex items-center justify-between px-4 py-2 border-b">
        <h1 className="text-lg font-semibold">Claude Fleet</h1>
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setSettingsOpen(true)}
          >
            <Settings className="h-4 w-4" />
          </Button>
        </div>
      </header>
      <TabHeader tabs={TABS} activeTab={activeTab} onTabChange={onTabChange} />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>

      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </div>
  )
}