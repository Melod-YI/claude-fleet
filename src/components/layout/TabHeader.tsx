import { cn } from "@/lib/utils"

interface Tab {
  id: string
  label: string
  count?: number
}

interface TabHeaderProps {
  tabs: Tab[]
  activeTab: string
  onTabChange: (tabId: string) => void
}

export function TabHeader({ tabs, activeTab, onTabChange }: TabHeaderProps) {
  return (
    <div className="flex items-center gap-1">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onTabChange(tab.id)}
          className={cn(
            "px-4 py-2 text-sm font-medium rounded-md transition-colors",
            "hover:bg-muted",
            activeTab === tab.id
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground"
          )}
        >
          {tab.label}
          {tab.count !== undefined && (
            <span className="ml-2 text-xs opacity-70">({tab.count})</span>
          )}
        </button>
      ))}
    </div>
  )
}