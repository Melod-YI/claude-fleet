import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogDescription,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { AlertTriangle } from "lucide-react"
import type { DeletionSafety } from "@/types"

interface DeleteWorktreeDialogProps {
  open: boolean
  worktreeName: string
  branch: string | null
  safety: DeletionSafety | null
  onClose: () => void
  onConfirm: () => void
}

export function DeleteWorktreeDialog({
  open,
  worktreeName,
  branch,
  safety,
  onClose,
  onConfirm,
}: DeleteWorktreeDialogProps) {
  if (!safety) return null

  const blocked = safety.blocked
  const willDeleteBranch = safety.willDeleteBranch

  const description = willDeleteBranch
    ? `将删除 worktree「${worktreeName}」的目录和分支「${branch ?? "--"}」，此操作不可撤销。`
    : `将删除 worktree「${worktreeName}」的目录（未托管，不删除分支），此操作不可撤销。`

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle className={blocked ? "text-destructive" : ""}>
            {blocked ? (
              <span className="flex items-center gap-1.5">
                <AlertTriangle className="w-4 h-4" />
                删除 Worktree - 存在风险
              </span>
            ) : (
              "删除 Worktree"
            )}
          </DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        {blocked && (
          <ul className="text-sm text-destructive space-y-1 my-2">
            {safety.reasons.map((r) => (
              <li key={r} className="flex items-center gap-1.5">
                <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                {r}
              </li>
            ))}
          </ul>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              onConfirm()
              onClose()
            }}
          >
            {blocked ? "我已知晓风险，强制删除" : "删除"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
