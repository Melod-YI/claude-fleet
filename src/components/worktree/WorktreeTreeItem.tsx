import { GitBranch } from "lucide-react"
import { cn } from "@/lib/utils"
import type { WorktreeListItem } from "@/types"

interface WorktreeTreeItemProps {
  worktree: WorktreeListItem
  isSelected: boolean
  onSelect: () => void
}

export function WorktreeTreeItem({
  worktree,
  isSelected,
  onSelect,
}: WorktreeTreeItemProps) {
  const isMissing = worktree.status === "missing"

  return (
    <div
      className={cn(
        "flex items-center gap-1.5 px-2 py-1 rounded cursor-pointer text-sm transition-colors",
        isSelected
          ? "bg-violet-100 border border-violet-300"
          : "hover:bg-accent/50",
        isMissing && "opacity-50"
      )}
      onClick={onSelect}
    >
      <GitBranch className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
      <span className="truncate">{worktree.name}</span>
    </div>
  )
}
