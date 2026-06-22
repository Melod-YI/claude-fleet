import { useMutation, useQueryClient } from "@tanstack/react-query"
import { toast } from "sonner"
import { worktreesApi } from "@/lib/api/worktrees"
import type { TrackedRepo } from "@/types"

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
    onError: (error: Error) => {
      if (error.message.includes("UNIQUE constraint")) {
        toast.error("该仓库已在列表中")
      } else {
        toast.error(`添加仓库失败: ${error.message}`)
      }
    },
  })
}

export const useRemoveTrackedRepoMutation = () => {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (id: number) => {
      await worktreesApi.removeTrackedRepo(id)
      return id
    },
    onSuccess: (id) => {
      queryClient.setQueryData<TrackedRepo[]>(["trackedRepos"], (current) =>
        (current ?? []).filter((repo) => repo.id !== id)
      )
      // Remove cached worktrees for this repo
      queryClient.removeQueries({ queryKey: ["worktrees"] })
      toast.success("已从列表中移除仓库")
    },
    onError: (error: Error) => {
      toast.error(`移除仓库失败: ${error.message}`)
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
    onError: (error: Error) => {
      toast.error(`创建 Worktree 失败: ${error.message}`)
    },
  })
}
