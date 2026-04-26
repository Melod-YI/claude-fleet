import { cn } from "@/lib/utils"

interface SplitPaneProps {
  left: React.ReactNode
  right: React.ReactNode
  leftWidth?: number | string
  className?: string
}

export function SplitPane({ left, right, leftWidth = 280, className }: SplitPaneProps) {
  return (
    <div className={cn("flex h-full overflow-hidden", className)}>
      <div
        style={{ width: leftWidth }}
        className="flex-shrink-0 border-r overflow-hidden"
      >
        {left}
      </div>
      <div className="flex-1 overflow-hidden">
        {right}
      </div>
    </div>
  )
}