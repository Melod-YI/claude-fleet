import { useState, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { worktreesApi } from "@/lib/api/worktrees"
import { open as openDialog } from "@tauri-apps/plugin-dialog"
import { RefreshCw } from "lucide-react"
import { useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/dialogs"
import { DeleteWorktreeDialog } from "./DeleteWorktreeDialog"
import { RepoTree } from "./RepoTree"
import { WorktreeDetail } from "./WorktreeDetail"
import { CreateWorktreeDialog } from "./CreateWorktreeDialog"
import { useTrackedReposQuery } from "@/lib/query/worktreeQueries"
import {
  useAddTrackedRepoMutation,
  useRemoveTrackedRepoMutation,
  useDeleteWorktreeMutation,
} from "@/lib/query/worktreeMutations"
import { startNewSession } from "@/services/sessionLaunchService"
import type { WorktreeListItem, DeletionSafety } from "@/types"

export function WorktreeTab() {
  const queryClient = useQueryClient()

  const [selectedWorktree, setSelectedWorktree] = useState<WorktreeListItem | null>(null)
  const [createDialogOpen, setCreateDialogOpen] = useState(false)
  const [createDialogRepoPath, setCreateDialogRepoPath] = useState("")
  const [removeRepoConfirm, setRemoveRepoConfirm] = useState<{
    open: boolean
    repoId: number
    repoName: string
  }>({ open: false, repoId: 0, repoName: "" })
  const [deleteConfirm, setDeleteConfirm] = useState<{
    open: boolean
    worktree: WorktreeListItem | null
    safety: DeletionSafety | null
  }>({ open: false, worktree: null, safety: null })

  const { data: trackedRepos = [] } = useTrackedReposQuery()

  const addRepoMutation = useAddTrackedRepoMutation()
  const removeRepoMutation = useRemoveTrackedRepoMutation()
  const deleteWorktreeMutation = useDeleteWorktreeMutation()

  const handleAddRepo = useCallback(async () => {
    try {
      const selected = await openDialog({ directory: true, multiple: false })
      if (!selected) return

      const path = selected as string
      const parts = path.split(/[\\/]/).filter(Boolean)
      const name = parts.pop() || "unknown"

      addRepoMutation.mutate({ path, name })
    } catch (e) {
      console.error("添加仓库失败:", e)
    }
  }, [addRepoMutation])

  const handleRemoveRepo = useCallback((repoId: number) => {
    const repo = trackedRepos.find((r) => r.id === repoId)
    if (repo) {
      setRemoveRepoConfirm({ open: true, repoId, repoName: repo.name })
    }
  }, [trackedRepos])

  const handleConfirmRemoveRepo = useCallback(() => {
    const removedRepo = trackedRepos.find((r) => r.id === removeRepoConfirm.repoId)
    if (removedRepo) {
      removeRepoMutation.mutate({ id: removedRepo.id, repoPath: removedRepo.path })
      // Clear selection if the selected worktree belongs to the removed repo
      if (selectedWorktree && selectedWorktree.path.startsWith(removedRepo.path)) {
        setSelectedWorktree(null)
      }
    }
    setRemoveRepoConfirm({ open: false, repoId: 0, repoName: "" })
  }, [removeRepoMutation, removeRepoConfirm, selectedWorktree, trackedRepos])

  const handleDeleteWorktree = useCallback(async (worktree: WorktreeListItem) => {
    const repo = trackedRepos.find(
      (r) => wtPathStartsWith(worktree.path, r.path)
    )
    const repoPath = repo?.path ?? worktree.path
    try {
      const safety = await worktreesApi.preflightDeleteWorktree(
        worktree.path,
        repoPath,
        worktree.branch ?? null
      )
      setDeleteConfirm({ open: true, worktree, safety })
    } catch (e) {
      console.error("删除预检失败:", e)
      const msg = e instanceof Error ? e.message : String(e)
      toast.error(`删除预检失败: ${msg}`)
    }
  }, [trackedRepos])

  const handleConfirmDelete = useCallback(() => {
    if (!deleteConfirm.worktree) return
    const wt = deleteConfirm.worktree
    const repo = trackedRepos.find((r) => wtPathStartsWith(wt.path, r.path))
    const repoPath = repo?.path ?? wt.path
    const deleteBranch = deleteConfirm.safety?.willDeleteBranch ?? false

    // 立即清空详情面板（不等 mutation 返回）
    if (selectedWorktree?.path === wt.path) {
      setSelectedWorktree(null)
    }

    deleteWorktreeMutation.mutate({
      path: wt.path,
      repoPath,
      branch: wt.branch ?? null,
      deleteBranch,
    })
    setDeleteConfirm({ open: false, worktree: null, safety: null })
  }, [deleteWorktreeMutation, deleteConfirm, selectedWorktree, trackedRepos])

  const handleAddWorktree = useCallback((repoPath: string) => {
    setCreateDialogRepoPath(repoPath)
    setCreateDialogOpen(true)
  }, [])

  const handleRefresh = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: ["worktrees"] })
  }, [queryClient])

  const handleLaunchClaude = useCallback(async (worktree: WorktreeListItem) => {
    try {
      await startNewSession({
        workingDirectory: worktree.path,
        name: worktree.name,
      })
    } catch (e) {
      console.error("启动 Claude Code 失败:", e)
    }
  }, [])

  const handleOpenDirectory = useCallback((path: string) => {
    invoke("open_directory", { path }).catch(console.error)
  }, [])

  const handleOpenVSCode = useCallback((path: string) => {
    invoke("open_in_vscode", { path }).catch(console.error)
  }, [])

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b bg-white">
        <h2 className="text-base font-semibold text-gray-900 shrink-0">Worktree</h2>
        <div className="w-px h-6 bg-gray-200" />
        <Button
          variant="outline"
          size="sm"
          onClick={handleRefresh}
          className="h-8"
          title="刷新"
        >
          <RefreshCw className="w-4 h-4" />
        </Button>
      </div>

      {/* Split layout */}
      <div className="flex-1 flex min-h-0 overflow-hidden">
        {/* Left sidebar */}
        <div className="w-[240px] min-w-[240px] border-r border-gray-200 bg-gray-50 overflow-hidden">
          <RepoTree
            repos={trackedRepos}
            selectedWorktreePath={selectedWorktree?.path ?? null}
            onSelectWorktree={setSelectedWorktree}
            onAddRepo={handleAddRepo}
            onRemoveRepo={handleRemoveRepo}
            onAddWorktree={handleAddWorktree}
          />
        </div>

        {/* Right detail panel */}
        <div className="flex-1 min-w-0 bg-white overflow-hidden">
          <WorktreeDetail
            worktree={selectedWorktree}
            onLaunchClaude={handleLaunchClaude}
            onOpenDirectory={handleOpenDirectory}
            onOpenVSCode={handleOpenVSCode}
            onDelete={handleDeleteWorktree}
          />
        </div>
      </div>

      {/* Create worktree dialog */}
      <CreateWorktreeDialog
        open={createDialogOpen}
        onClose={() => setCreateDialogOpen(false)}
        repoPath={createDialogRepoPath}
        onCreated={(worktreeInfo) => {
          // Auto-select the newly created worktree
          setSelectedWorktree({
            id: worktreeInfo.id,
            name: worktreeInfo.name,
            path: worktreeInfo.path,
            branch: worktreeInfo.branch,
            baseRef: worktreeInfo.baseRef,
            createdAt: worktreeInfo.createdAt,
            repoName: worktreeInfo.repoName,
            head: "",
            isMain: false,
            status: "active" as const,
          })
        }}
      />

      {/* Remove repo confirmation */}
      <ConfirmDialog
        open={removeRepoConfirm.open}
        onClose={() => setRemoveRepoConfirm({ open: false, repoId: 0, repoName: "" })}
        onConfirm={handleConfirmRemoveRepo}
        title="移除仓库"
        description={`将从列表中移除「${removeRepoConfirm.repoName}」，不会删除本地文件。`}
        confirmText="移除"
        variant="destructive"
      />

      {/* Delete worktree confirmation */}
      <DeleteWorktreeDialog
        open={deleteConfirm.open}
        worktreeName={deleteConfirm.worktree?.name ?? ""}
        branch={deleteConfirm.worktree?.branch ?? null}
        safety={deleteConfirm.safety}
        onClose={() => setDeleteConfirm({ open: false, worktree: null, safety: null })}
        onConfirm={handleConfirmDelete}
      />
    </div>
  )
}

/** 兼容正反斜杠的路径前缀匹配 */
function wtPathStartsWith(childPath: string, parentPath: string): boolean {
  return (
    childPath.startsWith(parentPath + "/") ||
    childPath.startsWith(parentPath + "\\")
  )
}
