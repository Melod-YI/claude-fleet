import { useEffect, useRef, useCallback } from 'react'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore } from '@/stores'
import { sendDesktopNotification, playNotificationSound } from '@/services'

interface WaitingInputEvent {
  session_id: string
  cwd?: string
  hook_event_name: string
}

export function useNotification() {
  const { notificationSound, notificationDesktop } = useSettingsStore()
  const notifiedSessions = useRef<Set<string>>(new Set())
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 发送通知
  const sendNotification = useCallback((sessionId: string, sessionName: string, cwd?: string) => {
    const fallbackName = cwd?.split(/[\\/]/).pop() || sessionId

    if (notificationDesktop) {
      sendDesktopNotification({
        title: 'Claude Fleet - 等待输入',
        body: `Session "${sessionName || fallbackName}" 正在等待输入`,
        sessionId,
        sound: notificationSound,
      })
    } else if (notificationSound) {
      playNotificationSound()
    }
  }, [notificationDesktop, notificationSound])

  // 监听等待输入事件
  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<WaitingInputEvent>('session_waiting_input', (event) => {
        const payload = event.payload

        // 检查是否已通知过
        if (!notifiedSessions.current.has(payload.session_id)) {
          notifiedSessions.current.add(payload.session_id)

          // 从 cwd 提取名称
          const name = payload.cwd?.split(/[\\/]/).pop() || ''
          sendNotification(payload.session_id, name, payload.cwd)
        }
      })
    }

    setupListener()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [sendNotification])

  // 清除通知记录（当 session 状态变化时）
  const clearNotifiedSession = useCallback((sessionId: string) => {
    notifiedSessions.current.delete(sessionId)
  }, [])

  return {
    clearNotifiedSession,
  }
}