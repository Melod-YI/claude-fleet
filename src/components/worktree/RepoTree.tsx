import { ScrollArea } from "@/components/ui/scroll-area"
import { RepoTreeItem } from "./RepoTreeItem"
import type { TrackedRepo, WorktreeListItem } from "@/types"

interface RepoTreeProps {
  repos: TrackedRepo[]
  selectedWorktreePath: string | null
  onSelectWorktree: (worktree: WorktreeListItem) => void
  onAddRepo: () => void
  onRemoveRepo: (repoId: number) => void
  onAddWorktree: (repoPath: string) => void
}

export function RepoTree({
  repos,
  selectedWorktreePath,
  onSelectWorktree,
  onAddRepo,
  onRemoveRepo,
  onAddWorktree,
}: RepoTreeProps) {
  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2 border-b">
        <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
          仓库列表
        </span>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-2 space-y-1">
          {repos.map((repo) => (
            <RepoTreeItem
              key={repo.id}
              repoId={repo.id}
              repoName={repo.name}
              repoPath={repo.path}
              selectedWorktreePath={selectedWorktreePath}
              onSelectWorktree={onSelectWorktree}
              onRemoveRepo={onRemoveRepo}
              onAddWorktree={onAddWorktree}
            />
          ))}

          <div
            className="text-center py-6 text-xs text-muted-foreground/50 border border-dashed border-muted-foreground/20 rounded cursor-pointer hover:text-muted-foreground/70 hover:border-muted-foreground/40 transition-colors"
            onClick={onAddRepo}
          >
            ＋ 添加仓库
          </div>
        </div>
      </ScrollArea>
    </div>
  )
}
