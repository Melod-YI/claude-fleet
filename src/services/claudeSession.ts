import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession, Conversation } from '@/types'
import type { RunningSession } from '@/hooks/useRunningSessions'

/**
 * 获取运行中 session 列表（轻量级，用于 Running Tab）
 * 使用新的增量状态管理
 */
export async function listRunningSessions(): Promise<RunningSession[]> {
  try {
    const sessions = await invoke<RunningSession[]>('list_running')
    return sessions
  } catch (error) {
    console.error('获取运行中 session 列表失败:', error)
    throw error
  }
}

/**
 * 获取所有 Claude session 列表（用于 Management Tab）
 */
export async function listSessions(): Promise<ClaudeSession[]> {
  try {
    const sessions = await invoke<ClaudeSession[]>('refresh_sessions')
    return sessions
  } catch (error) {
    console.error('获取 session 列表失败:', error)
    throw error
  }
}

/**
 * 获取指定 session 的对话内容
 */
export async function getConversation(sessionId: string): Promise<Conversation> {
  try {
    const conversation = await invoke<Conversation>('get_conversation', { sessionId })
    return conversation
  } catch (error) {
    console.error('获取对话内容失败:', error)
    throw error
  }
}

/**
 * 刷新 session 列表
 */
export async function refreshSessions(): Promise<ClaudeSession[]> {
  try {
    const sessions = await invoke<ClaudeSession[]>('refresh_sessions')
    return sessions
  } catch (error) {
    console.error('刷新 session 列表失败:', error)
    throw error
  }
}

/**
 * 删除 session
 */
export async function deleteSession(sessionId: string): Promise<void> {
  try {
    await invoke('delete_session_cmd', { sessionId })
  } catch (error) {
    console.error('删除 session 失败:', error)
    throw error
  }
}