// src/types/worktree.ts
// Worktree 相关类型定义

export interface RemoteInfo {
  name: string
  url: string
}

export interface RepoInfo {
  name: string
  remotes: RemoteInfo[]
  localBranches: string[]
  remoteBranches: string[]
  defaultBranch: string
}

export interface FetchResult {
  success: boolean
  message: string | null
}

export interface WorktreeInfo {
  id: number
  name: string
  branch: string
  path: string
  repoName: string
  repoPath: string
  baseRef: string
  createdAt: number
}

export type WorktreeStatus = 'active' | 'missing' | 'unmanaged'

export interface WorktreeListItem {
  id: number | null
  name: string
  repoName: string
  baseRef: string | null
  createdAt: number | null
  path: string
  head: string
  branch: string | null
  isMain: boolean
  status: WorktreeStatus
  ahead?: number
  behind?: number
  uncommittedChanges?: number
}

export interface DeletionSafety {
  isManaged: boolean
  willDeleteBranch: boolean
  uncommittedChanges: number
  unmergedCommits: number
  blocked: boolean
  reasons: string[]
}

export interface TrackedRepo {
  id: number
  path: string
  name: string
  addedAt: number
}
