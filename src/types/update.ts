export interface UpdateInfo {
  /** 最新版本号（如 "0.9.0"，不含 v 前缀） */
  latestVersion: string
  /** GitHub Release 页面 URL */
  releaseUrl: string
  /** Release notes（markdown 原文，可能为空） */
  releaseNotes?: string
  /** 发布时间（ISO 8601 字符串） */
  publishedAt: string
}
