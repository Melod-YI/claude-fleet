import { useState, useEffect, useRef } from "react"
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
import { cn } from "@/lib/utils"
import { Loader2, ChevronDown, ChevronRight, RefreshCw } from "lucide-react"
import { useRepoInfoQuery } from "@/lib/query/worktreeQueries"
import { useCreateWorktreeMutation, useFetchRepoRemotesMutation } from "@/lib/query/worktreeMutations"
import { getSetting, setSetting } from "@/services/dbService"
import { normalizePath } from "@/stores/settingsStore"
import type { WorktreeInfo } from "@/types"

interface CreateWorktreeDialogProps {
  open: boolean
  onClose: () => void
  repoPath: string
  onCreated?: (worktree: WorktreeInfo) => void
}

// Windows 路径非法字符
const ILLEGAL_CHARS = /[\\/:*?"<>|]/

// 按仓库记忆上次选择的 baseRef 的 setting key
const baseRefKey = (repoPath: string) => `worktree.baseRef.${normalizePath(repoPath)}`

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
  const [branchSearch, setBranchSearch] = useState("")
  const branchSearchRef = useRef<HTMLInputElement>(null)

  const { data: repoInfo, isLoading: repoInfoLoading, refetch: refetchRepoInfo, isFetching: repoInfoFetching } = useRepoInfoQuery(
    open ? repoPath : undefined
  )
  const createMutation = useCreateWorktreeMutation()
  const fetchMutation = useFetchRepoRemotesMutation()
  const [fetchError, setFetchError] = useState<string | null>(null)

  // Reset state when dialog opens
  useEffect(() => {
    if (!open) return
    setName("")
    setShowAdvanced(false)
    setCustomBranch("")
    setBranchSearch("")
    setFetchError(null)
    setBaseRef("")
    // 按仓库恢复上次选择的 baseRef（异步加载，加载后由 repoInfo effect 校验有效性）
    let cancelled = false
    getSetting(baseRefKey(repoPath))
      .then((saved) => {
        if (!cancelled && saved) setBaseRef(saved)
      })
      .catch(() => {})
    return () => { cancelled = true }
  }, [open, repoPath])

  // Set default baseRef when repoInfo loads (only if not already set or saved value invalid)
  useEffect(() => {
    if (!repoInfo) return

    // If baseRef is already set, validate it still exists in this repo
    if (baseRef) {
      const allBranches = [...repoInfo.remoteBranches, ...repoInfo.localBranches]
      if (allBranches.includes(baseRef) || baseRef === repoInfo.defaultBranch) {
        return // saved value is valid for this repo
      }
    }

    // Priority: upstream > origin > bare default branch
    const upstreamDefault = `upstream/${repoInfo.defaultBranch}`
    if (repoInfo.remoteBranches.includes(upstreamDefault)) {
      setBaseRef(upstreamDefault)
      return
    }
    const originDefault = `origin/${repoInfo.defaultBranch}`
    if (repoInfo.remoteBranches.includes(originDefault)) {
      setBaseRef(originDefault)
      return
    }
    setBaseRef(repoInfo.defaultBranch)
  }, [repoInfo, baseRef])

  const handleRefresh = async () => {
    setFetchError(null)
    try {
      const res = await fetchMutation.mutateAsync(repoPath)
      //无论 fetch 成功失败都刷新本地分支视图（失败时展示本地缓存）
      await refetchRepoInfo()
      if (!res.success) {
        setFetchError(res.message ?? "远端刷新失败")
      }
    } catch {
      // invoke 级传输错误：提示后端不可达（不刷新列表）
      setFetchError("无法连接后端")
    }
  }

  const handleCreate = async () => {
    if (!name.trim()) return

    try {
      // Persist the selected baseRef for this repo (next time)
      if (effectiveBaseRef) {
        setSetting(baseRefKey(repoPath), effectiveBaseRef).catch(() => {})
      }
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

  // Filter by search query (case-insensitive contains)
  const filteredBranchOptions = branchSearch.trim()
    ? branchOptions.filter((opt) =>
        opt.value.toLowerCase().includes(branchSearch.trim().toLowerCase())
      )
    : branchOptions

  // Compute effective values before JSX (used in summary bar and create handler)
  const effectiveBranch = showAdvanced && customBranch.trim()
    ? customBranch.trim()
    : name.trim()
  const effectiveBaseRef = baseRef || "main"

  // Check if the target branch already exists (case-sensitive, git is case-sensitive for branches)
  const branchExists = effectiveBranch
    ? branchOptions.some((opt) => opt.value === effectiveBranch)
    : false
  const branchError = branchExists
    ? `分支 "${effectiveBranch}" 已存在，请更换名称`
    : null

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
              autoComplete="off"
              onKeyDown={(e) => {
                if (e.key === "Enter" && name.trim() && !nameError && !branchExists) {
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
            <div className={cn(
              "border rounded-md px-3 py-2 text-sm",
              branchExists
                ? "bg-red-50 border-red-200 text-red-700"
                : "bg-violet-50 border-violet-200 text-violet-700"
            )}>
              分支：<span className="font-medium">{effectiveBranch}</span>
              {" · "}基于：<span className="font-medium">{effectiveBaseRef}</span>
            </div>
          )}
          {branchError && (
            <p className="text-xs text-destructive">{branchError}</p>
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
                  autoComplete="off"
                />
              </div>

              <div className="flex flex-col gap-2">
                <div className="flex items-center gap-2">
                  <Label className="text-sm">基于分支 / ref</Label>
                  <button
                    type="button"
                    onClick={handleRefresh}
                    disabled={fetchMutation.isPending || repoInfoFetching}
                    className="relative text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
                    title={fetchError ? `远端刷新失败：${fetchError}，显示为本地缓存` : "刷新分支列表"}
                  >
                    <RefreshCw
                      className={cn(
                        "w-3 h-3",
                        (fetchMutation.isPending || repoInfoFetching) && "animate-spin"
                      )}
                    />
                    {fetchError && (
                      <span className="absolute -top-0.5 -right-0.5 w-1.5 h-1.5 rounded-full bg-red-500" />
                    )}
                  </button>
                </div>
                {repoInfoLoading ? (
                  <div className="text-sm text-muted-foreground flex items-center gap-2">
                    <Loader2 className="w-3 h-3 animate-spin" />
                    加载分支列表...
                  </div>
                ) : (
                  <Select value={baseRef} onValueChange={setBaseRef} onOpenChange={(open) => {
                    if (open) {
                      setBranchSearch("")
                      // Auto-focus search input after SelectContent mounts
                      requestAnimationFrame(() => branchSearchRef.current?.focus())
                    }
                  }}>
                    <SelectTrigger className="w-full">
                      <SelectValue placeholder="选择基准分支" />
                    </SelectTrigger>
                    <SelectContent>
                      <div className="px-1 pb-1">
                        <Input
                          ref={branchSearchRef}
                          value={branchSearch}
                          onChange={(e) => setBranchSearch(e.target.value)}
                          placeholder="搜索分支..."
                          className="h-8 text-xs"
                          onKeyDown={(e) => {
                            // Prevent Radix Select from hijacking keyboard navigation
                            if (e.key !== "Escape" && e.key !== "Enter") {
                              e.stopPropagation()
                            }
                          }}
                        />
                      </div>
                      {filteredBranchOptions.length === 0 ? (
                        <div className="px-2 py-4 text-center text-xs text-muted-foreground">
                          未找到匹配的分支
                        </div>
                      ) : (
                        filteredBranchOptions.map((opt) => (
                          <SelectItem key={opt.value} value={opt.value}>
                            {opt.label}
                            {opt.group === "remote" && (
                              <span className="ml-2 text-xs text-muted-foreground">remote</span>
                            )}
                          </SelectItem>
                        ))
                      )}
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
            disabled={!name.trim() || !!nameError || branchExists || createMutation.isPending}
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
