import { invoke } from "@tauri-apps/api/core"
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo } from "@/types"

export const worktreesApi = {
  // Tracked repos
  async listTrackedRepos(): Promise<TrackedRepo[]> {
    return await invoke("list_tracked_repos_cmd")
  },

  async addTrackedRepo(path: string, name: string): Promise<TrackedRepo> {
    return await invoke("add_tracked_repo_cmd", { path, name })
  },

  async removeTrackedRepo(id: number): Promise<void> {
    return await invoke("remove_tracked_repo_cmd", { id })
  },

  // Worktrees
  async listWorktrees(repoPath: string): Promise<WorktreeListItem[]> {
    return await invoke("list_worktrees_cmd", { repoPath })
  },

  async createWorktree(
    repoPath: string,
    name: string,
    branch: string,
    baseRef: string
  ): Promise<WorktreeInfo> {
    return await invoke("create_worktree_cmd", { repoPath, name, branch, baseRef })
  },

  async deleteWorktree(
    path: string,
    repoPath: string,
    branch: string | null,
    deleteBranch: boolean
  ): Promise<void> {
    return await invoke("delete_worktree_cmd", { path, repoPath, branch, deleteBranch })
  },

  // Repo info
  async getRepoInfo(repoPath: string): Promise<RepoInfo> {
    return await invoke("get_repo_info_cmd", { repoPath })
  },
}
