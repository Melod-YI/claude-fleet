import { TabHeader } from "./TabHeader"

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
  return (
    <div className="flex flex-col h-screen bg-background">
      <header className="flex items-center justify-between px-4 py-2 border-b">
        <h1 className="text-lg font-semibold">Claude Fleet</h1>
        <div className="flex items-center gap-2">
          {/* 后续添加设置按钮 */}
        </div>
      </header>
      <TabHeader tabs={TABS} activeTab={activeTab} onTabChange={onTabChange} />
      <main className="flex-1 overflow-hidden">
        {children}
      </main>
    </div>
  )
}