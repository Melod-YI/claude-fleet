import { useState, useEffect, useCallback, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'

// Session 运行状态（对应 Claude JSON 文件中的三种状态）
export type SessionStatus = 'busy' | 'idle' | 'waiting'

export interface RunningSession {
  session_id: string
  pid: number
  status: SessionStatus
  cwd: string
  name: string
  updated_at: number
  away_summary?: string
  away_summary_at?: number
  last_user_input?: string
  custom_name?: string       // Claude Fleet 自定义名称
}

export function useRunningSessions() {
  const [sessions, setSessions] = useState<RunningSession[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 加载初始列表
  const loadSessions = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const result = await invoke<RunningSession[]>('list_running')
      setSessions(result)
    } catch (e) {
      setError(String(e))
    }
    setLoading(false)
  }, [])

  // 初始加载 + 监听事件
  useEffect(() => {
    loadSessions()

    // 监听状态变化事件
    const setupListener = async () => {
      unlistenRef.current = await listen<RunningSession[]>('running_sessions_changed', (event) => {
        setSessions(event.payload)
        setLoading(false)
      })
    }
    setupListener()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [loadSessions])

  // 手动刷新
  const refresh = useCallback(async () => {
    await loadSessions()
  }, [loadSessions])

  return {
    sessions,
    loading,
    error,
    refresh,
  }
}