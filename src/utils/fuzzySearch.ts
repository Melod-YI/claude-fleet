import type { ClaudeSession } from '@/types'

/**
 * 简单的模糊匹配函数
 * 检查 query 是否在 text 中出现（不区分大小写）
 */
export function fuzzyMatch(text: string, query: string): boolean {
  const lowerText = text.toLowerCase()
  const lowerQuery = query.toLowerCase()
  return lowerText.includes(lowerQuery)
}

/**
 * 搜索 session
 * 支持搜索名称、路径、对话内容
 */
export function searchSessions(
  sessions: ClaudeSession[],
  query: string,
  searchableFields: ('name' | 'path' | 'content')[] = ['name', 'path']
): ClaudeSession[] {
  if (!query.trim()) return sessions

  return sessions.filter((session) => {
    if (searchableFields.includes('name') && fuzzyMatch(session.name, query)) {
      return true
    }
    if (searchableFields.includes('path') && fuzzyMatch(session.workingDirectory, query)) {
      return true
    }
    // 对话内容搜索需要额外实现
    return false
  })
}