import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { worktreesApi } from "@/lib/api/worktrees"
import type { TrackedRepo } from "@/types"

/** Tauri invoke 可能抛出字符串而非 Error 对象，需要安全提取消息 */
function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === "string") return error
  return String(error)
}

export const useAddTrackedRepoMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ path, name }: { path: string; name: string }) => {
      return await worktreesApi.addTrackedRepo(path, name)
    },
    onSuccess: (repo: TrackedRepo) => {
      queryClient.setQueryData<TrackedRepo[]>(["trackedRepos"], (current) =>
        [repo, ...(current ?? [])]
      )
      toast.success(`已添加仓库: ${repo.name}`)
    },
    onError: (error: unknown) => {
      const msg = getErrorMessage(error)
      if (msg.includes("UNIQUE constraint")) {
        toast.error("该仓库已在列表中")
      } else {
        toast.error(`添加仓库失败: ${msg}`)
      }
    },
  })
}

export const useRemoveTrackedRepoMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ id, repoPath }: { id: number; repoPath: string }) => {
      await worktreesApi.removeTrackedRepo(id)
      return { id, repoPath }
    },
    onSuccess: ({ id, repoPath }) => {
      queryClient.setQueryData<TrackedRepo[]>(["trackedRepos"], (current) =>
        (current ?? []).filter((repo) => repo.id !== id)
      )
      // Remove cached worktrees only for this specific repo
      queryClient.removeQueries({ queryKey: ["worktrees", repoPath] })
      toast.success("已从列表中移除仓库")
    },
    onError: (error: unknown) => {
      toast.error(`移除仓库失败: ${getErrorMessage(error)}`)
    },
  })
}

export const useCreateWorktreeMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({
      repoPath,
      name,
      branch,
      baseRef,
    }: {
      repoPath: string
      name: string
      branch: string
      baseRef: string
    }) => {
      return await worktreesApi.createWorktree(repoPath, name, branch, baseRef)
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ["worktrees", variables.repoPath] })
      toast.success(`Worktree "${_data.name}" 创建成功`)
    },
    onError: (error: unknown) => {
      toast.error(`创建 Worktree 失败: ${getErrorMessage(error)}`)
    },
  })
}
