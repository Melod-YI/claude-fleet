import { useQuery } from "@tanstack/react-query"
import { worktreesApi } from "@/lib/api/worktrees"
import type { TrackedRepo, WorktreeListItem, RepoInfo } from "@/types"

export const useTrackedReposQuery = () => {
  return useQuery<TrackedRepo[]>({
    queryKey: ["trackedRepos"],
    queryFn: () => worktreesApi.listTrackedRepos(),
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })
}

export const useWorktreesQuery = (repoPath: string | undefined) => {
  return useQuery<WorktreeListItem[]>({
    queryKey: ["worktrees", repoPath],
    queryFn: () => worktreesApi.listWorktrees(repoPath!),
    enabled: Boolean(repoPath),
    staleTime: 30 * 1000,
  })
}

export const useRepoInfoQuery = (repoPath: string | undefined) => {
  return useQuery<RepoInfo>({
    queryKey: ["repoInfo", repoPath],
    queryFn: () => worktreesApi.getRepoInfo(repoPath!),
    enabled: Boolean(repoPath),
    staleTime: 5 * 60 * 1000,
  })
}
