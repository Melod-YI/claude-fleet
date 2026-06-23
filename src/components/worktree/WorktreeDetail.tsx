import { Play, FolderOpen, Code, Trash2, ChevronRight, GitBranch } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import type { WorktreeListItem } from "@/types"

interface WorktreeDetailProps {
  worktree: WorktreeListItem | null
  onLaunchClaude: (worktree: WorktreeListItem) => void
  onOpenDirectory: (path: string) => void
  onOpenVSCode: (path: string) => void
  onDelete: (worktree: WorktreeListItem) => void
}

const statusConfig = {
  active: { label: "Active", className: "bg-green-100 text-green-700 border-green-200" },
  missing: { label: "Missing", className: "bg-red-100 text-red-700 border-red-200" },
  unmanaged: { label: "Unmanaged", className: "bg-yellow-100 text-yellow-700 border-yellow-200" },
}

export function WorktreeDetail({
  worktree,
  onLaunchClaude,
  onOpenDirectory,
  onOpenVSCode,
  onDelete,
}: WorktreeDetailProps) {
  if (!worktree) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <ChevronRight className="w-8 h-8 text-muted-foreground/30 mb-2" />
        <p className="text-sm">选择一个 worktree 查看详情</p>
        <p className="text-xs text-muted-foreground/60 mt-1">或点击「新建 Worktree」创建</p>
      </div>
    )
  }

  const isMissing = worktree.status === "missing"
  const status = statusConfig[worktree.status]

  const formatDate = (ts: number | null) => {
    if (!ts) return "--"
    return new Date(ts).toLocaleString("zh-CN", { hour12: false })
  }

  return (
    <div className="flex flex-col h-full overflow-y-auto p-4">
      {/* Title + status */}
      <div className="flex items-center gap-2 mb-4">
        <h2 className="text-lg font-semibold">{worktree.name}</h2>
        <Badge variant="outline" className={cn("text-xs", status.className)}>
          {status.label}
        </Badge>
      </div>

      {/* Basic info */}
      <div className="bg-muted/40 rounded-lg p-3 mb-4 space-y-2 text-sm">
        <InfoRow label="路径" value={worktree.path} mono />
        <InfoRow label="分支" value={worktree.branch ?? "--"} />
        <InfoRow label="基于" value={worktree.baseRef ?? "--"} />
        <InfoRow label="创建时间" value={formatDate(worktree.createdAt)} />
      </div>

      {/* Git status */}
      <div className="bg-muted/40 rounded-lg p-3 mb-4">
        <h3 className="text-sm font-medium mb-2 flex items-center gap-1">
          <GitBranch className="w-3.5 h-3.5" />
          Git 状态
        </h3>
        <div className="flex gap-4 text-sm">
          <div className="flex items-center gap-1">
            <span className={cn(
              "tabular-nums",
              worktree.ahead != null && worktree.ahead > 0
                ? "text-green-600 font-medium"
                : "text-muted-foreground"
            )}>
              {worktree.ahead ?? "--"}
            </span>
            <span className="text-muted-foreground/60">ahead</span>
          </div>
          <div className="flex items-center gap-1">
            <span className={cn(
              "tabular-nums",
              worktree.behind != null && worktree.behind > 0
                ? "text-orange-500 font-medium"
                : "text-muted-foreground"
            )}>
              {worktree.behind ?? "--"}
            </span>
            <span className="text-muted-foreground/60">behind</span>
          </div>
          <div className="flex items-center gap-1">
            <span className={cn(
              "tabular-nums",
              worktree.uncommittedChanges != null && worktree.uncommittedChanges > 0
                ? "text-red-500 font-medium"
                : "text-muted-foreground"
            )}>
              {worktree.uncommittedChanges ?? "--"}
            </span>
            <span className="text-muted-foreground/60">未提交变更</span>
          </div>
        </div>
      </div>

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2 mt-auto pt-4 border-t">
        <Button
          variant="default"
          size="sm"
          disabled={isMissing}
          onClick={() => onLaunchClaude(worktree)}
          className="bg-violet-600 hover:bg-violet-700"
        >
          <Play className="w-4 h-4 mr-1" />
          运行 Claude Code
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={isMissing}
          onClick={() => onOpenDirectory(worktree.path)}
        >
          <FolderOpen className="w-4 h-4 mr-1" />
          打开目录
        </Button>
        <Button
          variant="outline"
          size="sm"
          disabled={isMissing}
          onClick={() => onOpenVSCode(worktree.path)}
        >
          <Code className="w-4 h-4 mr-1" />
          VS Code
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => onDelete(worktree)}
        >
          <Trash2 className="w-4 h-4 mr-1" />
          删除
        </Button>
      </div>
    </div>
  )
}

function InfoRow({
  label,
  value,
  mono,
}: {
  label: string
  value: string
  mono?: boolean
}) {
  return (
    <div className="flex items-start gap-2">
      <span className="text-muted-foreground w-16 shrink-0">{label}</span>
      <span className={cn("truncate", mono && "font-mono text-xs")}>{value}</span>
    </div>
  )
}
