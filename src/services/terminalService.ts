import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession } from '@/types'

/**
 * 跳转到终端窗口
 * 查找并激活与指定 session 关联的 Windows Terminal 窗口
 */
export async function jumpToTerminal(session: ClaudeSession): Promise<void> {
  try {
    await invoke('jump_to_terminal', {
      workingDirectory: session.workingDirectory,
    })
  } catch (error) {
    // 失败时，复制路径到剪贴板作为备用方案
    await navigator.clipboard.writeText(session.workingDirectory)
    throw new Error(`跳转失败，路径已复制到剪贴板: ${error}`)
  }
}

/**
 * 在终端中恢复 session
 * 启动新的终端窗口并执行 claude --resume 命令
 */
export async function resumeInTerminal(session: ClaudeSession): Promise<void> {
  try {
    await invoke('resume_in_terminal', {
      workingDirectory: session.workingDirectory,
      sessionId: session.id,
    })
  } catch (error) {
    // 失败时，复制恢复命令作为备用方案
    const command = `claude --resume ${session.id}`
    await navigator.clipboard.writeText(command)
    throw new Error(`恢复失败，命令已复制到剪贴板: ${error}`)
  }
}