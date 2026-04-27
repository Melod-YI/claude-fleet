import { useEffect, useRef, useCallback } from 'react'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import { useSettingsStore, useSessionStore } from '@/stores'
import { sendDesktopNotification, initNotificationService, playNotificationSound } from '@/services'

interface HookEvent {
  event: string  // "start", "idle", "stop", "end"
  session_id: string
  cwd?: string
}

export function useNotification() {
  const { notificationSound, notificationDesktop } = useSettingsStore()
  const { sessions, refresh } = useSessionStore()
  const notifiedSessions = useRef<Set<string>>(new Set())
  const unlistenRef = useRef<UnlistenFn | null>(null)

  // 初始化通知服务
  useEffect(() => {
    initNotificationService()
  }, [])

  // 发送通知的函数
  const sendNotification = useCallback((sessionId: string, sessionName: string, cwd?: string) => {
    // 从 cwd 提取目录名作为备用名称
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

  // 监听来自 Rust 的钩子事件（文件监听方式）
  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<HookEvent>('hook_event', (event) => {
        const payload = event.payload

        console.log('收到钩子事件:', payload.event, payload.session_id)

        if (payload.event === 'idle') {
          // 等待用户输入 - 检查是否已经通知过
          if (!notifiedSessions.current.has(payload.session_id)) {
            notifiedSessions.current.add(payload.session_id)

            // 找到对应的 session 获取名称
            const session = sessions.find((s) => s.id === payload.session_id)
            sendNotification(payload.session_id, session?.name || '', payload.cwd)
          }
        } else if (payload.event === 'stop') {
          // Claude 完成响应 - 清除等待状态记录，刷新 session 列表
          notifiedSessions.current.delete(payload.session_id)
          // 可选：刷新列表以获取最新状态
          refresh()
        } else if (payload.event === 'end') {
          // Session 结束 - 清除通知记录
          notifiedSessions.current.delete(payload.session_id)
          refresh()
        } else if (payload.event === 'start') {
          // Session 启动 - 刷新列表
          refresh()
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
  }, [sessions, sendNotification, refresh])

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