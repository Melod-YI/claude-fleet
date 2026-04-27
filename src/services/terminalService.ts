import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession } from '@/types'

/**
 * 智能跳转到终端窗口
 * 先通过进程 ID 精确匹配，失败则通过路径匹配
 */
export async function jumpToTerminal(session: ClaudeSession): Promise<void> {
  try {
    await invoke('smart_jump_to_terminal', {
      workingDirectory: session.workingDirectory,
      processId: session.processId,
    })
  } catch (error) {
    // 失败时，复制路径到剪贴板作为备用方案
    await navigator.clipboard.writeText(session.workingDirectory)
    throw new Error(`跳转失败，路径已复制到剪贴板: ${error}`)
  }
}

/**
 * 通过进程 ID 精确跳转到终端窗口
 */
export async function jumpToTerminalByPid(processId: number): Promise<void> {
  try {
    await invoke('jump_to_terminal_by_pid', {
      processId,
    })
  } catch (error) {
    throw new Error(`跳转失败: ${error}`)
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