import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession, Conversation } from '@/types'

/**
 * 获取所有 Claude session 列表
 */
export async function listSessions(): Promise<ClaudeSession[]> {
  try {
    const sessions = await invoke<ClaudeSession[]>('list_sessions')
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