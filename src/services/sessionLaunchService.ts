import { invoke } from '@tauri-apps/api/core'
import type { ClaudeSession, LaunchSettings } from '@/types'
import { useSettingsStore } from '@/stores/settingsStore'

type LaunchMode =
  | { new: { name?: string } }
  | { resume: { sessionId: string } }

interface LaunchRequest {
  workingDirectory: string
  mode: LaunchMode
  settings: LaunchSettings
}

interface StartNewSessionOptions {
  workingDirectory: string
  name?: string
}

export async function startNewSession(options: StartNewSessionOptions): Promise<void> {
  await invoke('launch_session', {
    request: {
      workingDirectory: options.workingDirectory,
      mode: { new: { name: options.name } },
      settings: useSettingsStore.getState().launchSettings,
    } satisfies LaunchRequest,
  })
}

export async function resumeSessionInTerminal(session: ClaudeSession): Promise<void> {
  const settings = useSettingsStore.getState().launchSettings

  try {
    await invoke('launch_session', {
      request: {
        workingDirectory: session.workingDirectory,
        mode: { resume: { sessionId: session.id } },
        settings,
      } satisfies LaunchRequest,
    })
  } catch (error) {
    await navigator.clipboard.writeText(buildResumeCommand(session.id, settings))
    throw new Error(`恢复失败，命令已复制到剪贴板: ${error}`)
  }
}

export function buildResumeCommand(sessionId: string, settings: LaunchSettings): string {
  return commandLine(buildProcessArgv(settings, ['--resume', sessionId]))
}

function buildProcessArgv(settings: LaunchSettings, modeArgs: string[]): string[] {
  const agentArgv = [
    settings.claudeExecutable || 'claude',
    ...modeArgs,
    ...settings.claudeArgs,
  ]

  if (settings.wrapper?.enabled && settings.wrapper.executable.trim()) {
    return [
      settings.wrapper.executable,
      ...settings.wrapper.argsBeforeAgent,
      ...agentArgv,
    ]
  }

  return agentArgv
}

function commandLine(argv: string[]): string {
  return argv.map(quoteArg).join(' ')
}

function quoteArg(arg: string): string {
  if (!arg) return '""'
  if (!/\s|["']/.test(arg)) return arg
  return `"${arg.replace(/"/g, '\\"')}"`
}
