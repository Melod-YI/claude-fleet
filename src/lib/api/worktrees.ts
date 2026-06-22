import { invoke } from "@tauri-apps/api/core"
import type { TrackedRepo, WorktreeListItem, WorktreeInfo, RepoInfo } from "@/types"

export const worktreesApi = {
  // Tracked repos
  async listTrackedRepos(): Promise<TrackedRepo[]> {
    return await invoke("list_tracked_repos")
  },

  async addTrackedRepo(path: string, name: string): Promise<TrackedRepo> {
    console.log("[api] addTrackedRepo 调用 invoke:", { path, name })
    const result = await invoke("add_tracked_repo", { path, name })
    console.log("[api] addTrackedRepo 返回:", result)
    return result as TrackedRepo
  },

  async removeTrackedRepo(id: number): Promise<void> {
    return await invoke("remove_tracked_repo", { id })
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

  // Repo info
  async getRepoInfo(repoPath: string): Promise<RepoInfo> {
    return await invoke("get_repo_info_cmd", { repoPath })
  },
}
