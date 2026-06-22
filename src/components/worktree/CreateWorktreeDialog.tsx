import { useState, useEffect } from "react"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Loader2, ChevronDown, ChevronRight } from "lucide-react"
import { useRepoInfoQuery } from "@/lib/query/worktreeQueries"
import { useCreateWorktreeMutation } from "@/lib/query/worktreeMutations"
import type { WorktreeInfo } from "@/types"

interface CreateWorktreeDialogProps {
  open: boolean
  onClose: () => void
  repoPath: string
  onCreated?: (worktree: WorktreeInfo) => void
}

// Windows 路径非法字符
const ILLEGAL_CHARS = /[\\/:*?"<>|]/

export function CreateWorktreeDialog({
  open,
  onClose,
  repoPath,
  onCreated,
}: CreateWorktreeDialogProps) {
  const [name, setName] = useState("")
  const [showAdvanced, setShowAdvanced] = useState(false)
  const [customBranch, setCustomBranch] = useState("")
  const [baseRef, setBaseRef] = useState("")

  const { data: repoInfo, isLoading: repoInfoLoading } = useRepoInfoQuery(
    open ? repoPath : undefined
  )
  const createMutation = useCreateWorktreeMutation()

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setName("")
      setShowAdvanced(false)
      setCustomBranch("")
      setBaseRef("")
    }
  }, [open])

  // Set default baseRef when repoInfo loads
  useEffect(() => {
    if (repoInfo && !baseRef) {
      const originDefault = `origin/${repoInfo.defaultBranch}`
      const hasOriginDefault = repoInfo.remoteBranches.includes(originDefault)
      setBaseRef(hasOriginDefault ? originDefault : repoInfo.defaultBranch)
    }
  }, [repoInfo, baseRef])

  const handleCreate = async () => {
    if (!name.trim()) return

    try {
      const result = await createMutation.mutateAsync({
        repoPath,
        name: name.trim(),
        branch: effectiveBranch,
        baseRef: effectiveBaseRef,
      })
      onCreated?.(result)
      onClose()
    } catch {
      // Error handled by mutation's onError
    }
  }

  const nameError = name.trim()
    ? ILLEGAL_CHARS.test(name.trim())
      ? "名称包含非法字符 (\\ / : * ? \" < > |)"
      : null
    : null

  // Build branch options grouped by source
  const branchOptions = repoInfo
    ? [
        ...repoInfo.remoteBranches.map((b) => ({ value: b, label: b, group: "remote" })),
        ...repoInfo.localBranches
          .filter((b) => !repoInfo.remoteBranches.includes(b))
          .map((b) => ({ value: b, label: b, group: "local" })),
      ]
    : []

  // Compute effective values before JSX (used in summary bar and create handler)
  const effectiveBranch = showAdvanced && customBranch.trim()
    ? customBranch.trim()
    : name.trim()
  const effectiveBaseRef = baseRef || "main"

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>新建 Worktree</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4 py-4">
          {/* Name input */}
          <div className="flex flex-col gap-2">
            <Label htmlFor="wt-name">Worktree 名称</Label>
            <Input
              id="wt-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="例如: feature-auth"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && name.trim() && !nameError) {
                  handleCreate()
                }
              }}
            />
            {nameError && (
              <p className="text-xs text-destructive">{nameError}</p>
            )}
          </div>

          {/* Auto-config summary */}
          {name.trim() && (
            <div className="bg-violet-50 border border-violet-200 rounded-md px-3 py-2 text-sm text-violet-700">
              分支：<span className="font-medium">{effectiveBranch}</span>
              {" · "}基于：<span className="font-medium">{effectiveBaseRef}</span>
            </div>
          )}

          {/* Advanced toggle */}
          <button
            type="button"
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            {showAdvanced ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
            高级选项
          </button>

          {/* Advanced options */}
          {showAdvanced && (
            <div className="flex flex-col gap-3 pl-5 border-l-2 border-muted">
              <div className="flex flex-col gap-2">
                <Label htmlFor="wt-branch" className="text-sm">
                  分支名 <span className="text-muted-foreground font-normal">(留空则同名称)</span>
                </Label>
                <Input
                  id="wt-branch"
                  value={customBranch}
                  onChange={(e) => setCustomBranch(e.target.value)}
                  placeholder="自动"
                />
              </div>

              <div className="flex flex-col gap-2">
                <Label className="text-sm">基于分支 / ref</Label>
                {repoInfoLoading ? (
                  <div className="text-sm text-muted-foreground flex items-center gap-2">
                    <Loader2 className="w-3 h-3 animate-spin" />
                    加载分支列表...
                  </div>
                ) : (
                  <Select value={baseRef} onValueChange={setBaseRef}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="选择基准分支" />
                    </SelectTrigger>
                    <SelectContent>
                      {branchOptions.map((opt) => (
                        <SelectItem key={opt.value} value={opt.value}>
                          {opt.label}
                          {opt.group === "remote" && (
                            <span className="ml-2 text-xs text-muted-foreground">remote</span>
                          )}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            取消
          </Button>
          <Button
            variant="default"
            onClick={handleCreate}
            disabled={!name.trim() || !!nameError || createMutation.isPending}
            className="bg-violet-600 hover:bg-violet-700"
          >
            {createMutation.isPending ? (
              <>
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                创建中...
              </>
            ) : (
              "创建"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
