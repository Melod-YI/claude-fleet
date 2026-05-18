import type { SessionMeta, RunningSession } from '@/types'

/**
 * 获取 session 显示名称（优先级：customName > title > projectDir > sessionId）
 */
export function getDisplayName(session: SessionMeta | RunningSession): string {
  // RunningSession 使用下划线命名，SessionMeta 使用驼峰命名
  const customName = 'custom_name' in session ? session.custom_name : ('customName' in session ? session.customName : undefined)
  const title = 'title' in session ? session.title : ('name' in session ? session.name : undefined)
  const projectDir = 'cwd' in session ? session.cwd : session.projectDir
  const sessionId = 'session_id' in session ? session.session_id : session.sessionId

  return customName
    || title
    || projectDir?.split(/[\\/]/).pop()
    || sessionId.slice(0, 8)
}

export * from './fuzzySearch'
export * from './timeUtils'
export * from './pathUtils'