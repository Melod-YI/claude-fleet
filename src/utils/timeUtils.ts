import type { ClaudeSession } from '@/types'

const TIME_RANGES = {
  '3d': 3 * 24 * 60 * 60 * 1000,
  '7d': 7 * 24 * 60 * 60 * 1000,
  '30d': 30 * 24 * 60 * 60 * 1000,
  'all': Infinity,
}

/**
 * 过滤指定时间范围内的 session
 */
export function filterByTimeRange(
  sessions: ClaudeSession[],
  timeRange: '3d' | '7d' | '30d' | 'all'
): ClaudeSession[] {
  if (timeRange === 'all') return sessions

  const now = new Date().getTime()
  const rangeMs = TIME_RANGES[timeRange]

  return sessions.filter((session) => {
    const lastActivity = new Date(session.lastActivityAt).getTime()
    return now - lastActivity <= rangeMs
  })
}

/**
 * 格式化相对时间
 */
export function formatRelativeTime(dateString: string): string {
  const date = new Date(dateString)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()

  const minutes = Math.floor(diffMs / (60 * 1000))
  const hours = Math.floor(diffMs / (60 * 60 * 1000))
  const days = Math.floor(diffMs / (24 * 60 * 60 * 1000))

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  if (hours < 24) return `${hours} 小时前`
  if (days < 7) return `${days} 天前`
  if (days < 30) return `${Math.floor(days / 7)} 周前`

  return date.toLocaleDateString('zh-CN')
}

/**
 * 格式化相对时间（从毫秒 timestamp）
 */
export function formatRelativeTimeFromTimestamp(timestamp: number): string {
  const date = new Date(timestamp) // 已是毫秒，无需转换
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()

  const minutes = Math.floor(diffMs / (60 * 1000))
  const hours = Math.floor(diffMs / (60 * 60 * 1000))
  const days = Math.floor(diffMs / (24 * 60 * 60 * 1000))

  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  if (hours < 24) return `${hours} 小时前`
  if (days < 7) return `${days} 天前`
  if (days < 30) return `${Math.floor(days / 7)} 周前`

  return date.toLocaleDateString('zh-CN')
}