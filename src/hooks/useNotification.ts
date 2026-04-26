import { useEffect, useRef, useCallback } from 'react'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore, useSessionStore } from '@/stores'
import { sendDesktopNotification, initNotificationService, playNotificationSound } from '@/services'

interface HookEvent {
  event_type: string
  session_id: string
  working_directory: string
  timestamp: string
}

export function useNotification() {
  const { notificationSound, notificationDesktop } = useSettingsStore()
  const { sessions } = useSessionStore()
  const notifiedSessions = useRef<Set<string>>(new Set())
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 初始化通知服务
  useEffect(() => {
    initNotificationService()
  }, [])

  // 发送通知的函数
  const sendNotification = useCallback((sessionId: string, sessionName: string) => {
    if (notificationDesktop) {
      sendDesktopNotification({
        title: 'Claude Fleet - 等待输入',
        body: `Session "${sessionName}" 正在等待输入`,
        sessionId,
        sound: notificationSound,
      })
    } else if (notificationSound) {
      playNotificationSound()
    }
  }, [notificationDesktop, notificationSound])

  // 监听来自 Rust 的钩子事件
  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<HookEvent>('hook_event', (event) => {
        const payload = event.payload

        if (payload.event_type === 'waiting_input') {
          // 检查是否已经通知过此 session
          if (!notifiedSessions.current.has(payload.session_id)) {
            notifiedSessions.current.add(payload.session_id)

            // 找到对应的 session 获取名称
            const session = sessions.find((s) => s.id === payload.session_id)
            const sessionName = session?.name || payload.working_directory.split(/[\\/]/).pop() || payload.session_id

            sendNotification(payload.session_id, sessionName)
          }
        } else if (payload.event_type === 'session_end') {
          // Session 结束，清除通知记录
          notifiedSessions.current.delete(payload.session_id)
        } else if (payload.event_type === 'session_start') {
          // Session 启动，清除之前的通知记录（如果是重启）
          notifiedSessions.current.delete(payload.session_id)
        }
      })
    }

    setupListener()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
        unlistenRef.current = null
      }
    }
  }, [sessions, sendNotification])

  // 定期检查 session 状态（备用方案）
  useEffect(() => {
    const checkInterval = setInterval(() => {
      for (const session of sessions) {
        if (session.status === 'running') {
          // 状态为 running 时，检查是否需要通知
          if (!notifiedSessions.current.has(session.id)) {
            notifiedSessions.current.add(session.id)

            // 检查是否启用了通知
            if (notificationDesktop || notificationSound) {
              sendNotification(session.id, session.name)
            }
          }
        } else {
          // 状态变化（不再是 running），清除通知记录
          notifiedSessions.current.delete(session.id)
        }
      }
    }, 5000) // 每 5 秒检查一次

    return () => clearInterval(checkInterval)
  }, [sessions, notificationSound, notificationDesktop, sendNotification])

  // 手动触发通知的方法
  const notify = useCallback((sessionId: string, customMessage?: string) => {
    const session = sessions.find((s) => s.id === sessionId)
    if (session) {
      if (notificationDesktop) {
        sendDesktopNotification({
          title: 'Claude Fleet',
          body: customMessage || `Session "${session.name}"`,
          sessionId,
          sound: notificationSound,
        })
      } else if (notificationSound) {
        playNotificationSound()
      }
    }
  }, [sessions, notificationDesktop, notificationSound])

  // 测试通知
  const testNotification = useCallback(() => {
    sendDesktopNotification({
      title: 'Claude Fleet - 测试通知',
      body: '这是一条测试通知消息',
      sound: notificationSound,
    })
  }, [notificationSound])

  return {
    notify,
    testNotification,
    // 清除通知记录
    clearNotifiedSession: (sessionId: string) => {
      notifiedSessions.current.delete(sessionId)
    },
  }
}