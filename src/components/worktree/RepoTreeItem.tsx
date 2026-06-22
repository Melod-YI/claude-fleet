import { useState } from "react"
import { ChevronDown, ChevronRight, Folder, X } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { WorktreeTreeItem } from "./WorktreeTreeItem"
import { useWorktreesQuery } from "@/lib/query/worktreeQueries"
import type { WorktreeListItem } from "@/types"

interface RepoTreeItemProps {
  repoName: string
  repoPath: string
  repoId: number
  selectedWorktreePath: string | null
  onSelectWorktree: (worktree: WorktreeListItem) => void
  onRemoveRepo: (repoId: number) => void
  onAddWorktree: (repoPath: string) => void
}

export function RepoTreeItem({
  repoName,
  repoPath,
  repoId,
  selectedWorktreePath,
  onSelectWorktree,
  onRemoveRepo,
  onAddWorktree,
}: RepoTreeItemProps) {
  const [expanded, setExpanded] = useState(false)
  const [hovered, setHovered] = useState(false)

  // Each RepoTreeItem owns its own worktree query, enabled only when expanded
  const { data: worktrees = [], isLoading: worktreesLoading } = useWorktreesQuery(
    expanded ? repoPath : undefined
  )

  return (
    <div>
      {/* Repo header */}
      <div
        className={cn(
          "flex items-center gap-1.5 px-2 py-1.5 rounded cursor-pointer text-sm",
          "hover:bg-accent/50 transition-colors",
          expanded && "bg-accent/30"
        )}
        onClick={() => setExpanded(!expanded)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        {expanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        )}
        <Folder className="w-4 h-4 text-violet-500 shrink-0" />
        <span className="truncate font-medium">{repoName}</span>
        <span className="ml-auto text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
          {worktrees.length}
        </span>
        {hovered && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 shrink-0 opacity-50 hover:opacity-100 hover:text-destructive"
            onClick={(e) => {
              e.stopPropagation()
              onRemoveRepo(repoId)
            }}
            title="移除仓库"
          >
            <X className="w-3 h-3" />
          </Button>
        )}
      </div>

      {/* Worktree children */}
      {expanded && (
        <div className="ml-5 mt-0.5 space-y-0.5">
          {worktreesLoading ? (
            <div className="text-xs text-muted-foreground px-2 py-1">加载中...</div>
          ) : worktrees.length === 0 ? (
            <div className="text-xs text-muted-foreground px-2 py-1">暂无 worktree</div>
          ) : (
            worktrees.map((wt) => (
              <WorktreeTreeItem
                key={wt.path}
                worktree={wt}
                isSelected={selectedWorktreePath === wt.path}
                onSelect={() => onSelectWorktree(wt)}
              />
            ))
          )}
          <button
            className="w-full text-xs text-muted-foreground/60 hover:text-muted-foreground
                       border border-dashed border-muted-foreground/20 hover:border-muted-foreground/40
                       rounded px-2 py-1 mt-1 transition-colors"
            onClick={() => onAddWorktree(repoPath)}
          >
            ＋ 新建 worktree
          </button>
        </div>
      )}
    </div>
  )
}
