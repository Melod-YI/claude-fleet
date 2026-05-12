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
  const { notificationSound, notificationDesktop, notificationSoundFile } = useSettingsStore()
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
        soundFile: notificationSoundFile,
      })
    } else if (notificationSound) {
      playNotificationSound(notificationSoundFile)
    }
  }, [notificationDesktop, notificationSound, notificationSoundFile])

  // 监听等待输入事件
  // 后端已确保只在状态从 busy → idle/waiting 时发送一次事件，无需前端防重复
  useEffect(() => {
    const setupListener = async () => {
      unlistenRef.current = await listen<WaitingInputEvent>('session_waiting_input', (event) => {
        const payload = event.payload
        // 从 cwd 提取名称
        const name = payload.cwd?.split(/[\\/]/).pop() || ''
        sendNotification(payload.session_id, name, payload.cwd)
      })
    }

    setupListener()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [sendNotification])
}