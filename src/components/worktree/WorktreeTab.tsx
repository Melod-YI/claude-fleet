import { useState, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import { open as openDialog } from "@tauri-apps/plugin-dialog"
import { Plus, RefreshCw } from "lucide-react"
import { useQueryClient } from "@tanstack/react-query"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/dialogs"
import { RepoTree } from "./RepoTree"
import { WorktreeDetail } from "./WorktreeDetail"
import { CreateWorktreeDialog } from "./CreateWorktreeDialog"
import { useTrackedReposQuery } from "@/lib/query/worktreeQueries"
import {
  useAddTrackedRepoMutation,
  useRemoveTrackedRepoMutation,
} from "@/lib/query/worktreeMutations"
import { startNewSession } from "@/services/sessionLaunchService"
import type { WorktreeListItem } from "@/types"

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

  const { data: trackedRepos = [] } = useTrackedReposQuery()

  const addRepoMutation = useAddTrackedRepoMutation()
  const removeRepoMutation = useRemoveTrackedRepoMutation()

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
    removeRepoMutation.mutate(removeRepoConfirm.repoId)
    if (selectedWorktree) {
      const removedRepo = trackedRepos.find((r) => r.id === removeRepoConfirm.repoId)
      if (removedRepo && selectedWorktree.repoName === removedRepo.name) {
        setSelectedWorktree(null)
      }
    }
    setRemoveRepoConfirm({ open: false, repoId: 0, repoName: "" })
  }, [removeRepoMutation, removeRepoConfirm, selectedWorktree, trackedRepos])

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
          variant="default"
          size="sm"
          onClick={() => {
            if (trackedRepos.length > 0) {
              setCreateDialogRepoPath(trackedRepos[0].path)
              setCreateDialogOpen(true)
            } else {
              handleAddRepo()
            }
          }}
          className="h-8 bg-violet-600 hover:bg-violet-700"
        >
          <Plus className="w-4 h-4 mr-1" />
          新建 Worktree
        </Button>
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
            onDelete={() => {}}
          />
        </div>
      </div>

      {/* Create worktree dialog */}
      <CreateWorktreeDialog
        open={createDialogOpen}
        onClose={() => setCreateDialogOpen(false)}
        repoPath={createDialogRepoPath}
        onCreated={() => {
          // Query invalidation handled by mutation's onSuccess
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
    </div>
  )
}
