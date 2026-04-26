import { invoke } from '@tauri-apps/api/core'

// 提示音 URL（使用 base64 编码的简单提示音）
const NOTIFICATION_SOUND_URL = '/sounds/notification.mp3'

// 预加载提示音对象
let notificationSound: HTMLAudioElement | null = null

export interface NotificationOptions {
  title: string
  body: string
  sessionId?: string
  sound?: boolean
}

/**
 * 初始化音频对象
 */
function initAudio(): void {
  if (!notificationSound) {
    try {
      notificationSound = new Audio(NOTIFICATION_SOUND_URL)
      notificationSound.volume = 0.5
      notificationSound.preload = 'auto'
    } catch (e) {
      console.warn('初始化音频失败:', e)
    }
  }
}

/**
 * 播放提示音
 */
export function playNotificationSound(): void {
  initAudio()
  if (notificationSound) {
    notificationSound.currentTime = 0
    notificationSound.play().catch((e) => {
      console.warn('播放提示音失败:', e)
    })
  }
}

/**
 * 发送桌面通知
 */
export async function sendDesktopNotification(options: NotificationOptions): Promise<void> {
  try {
    // 尝试使用 Tauri 命令发送通知
    await invoke('send_notification', {
      title: options.title,
      body: options.body,
    })
  } catch (e) {
    // 降级：使用 Web Notifications API
    if ('Notification' in window) {
      if (Notification.permission === 'granted') {
        new Notification(options.title, {
          body: options.body,
          icon: '/favicon.ico',
        })
      } else if (Notification.permission !== 'denied') {
        const permission = await Notification.requestPermission()
        if (permission === 'granted') {
          new Notification(options.title, {
            body: options.body,
            icon: '/favicon.ico',
          })
        }
      }
    }
  }

  // 播放提示音
  if (options.sound) {
    playNotificationSound()
  }
}

/**
 * 请求通知权限
 */
export async function requestNotificationPermission(): Promise<boolean> {
  if ('Notification' in window) {
    const permission = await Notification.requestPermission()
    return permission === 'granted'
  }
  return false
}

/**
 * 检查通知权限状态
 */
export function getNotificationPermissionStatus(): 'granted' | 'denied' | 'default' {
  if ('Notification' in window) {
    return Notification.permission
  }
  return 'denied'
}

/**
 * 初始化通知服务
 */
export async function initNotificationService(): Promise<void> {
  // 初始化音频
  initAudio()

  // 请求通知权限（不阻塞）
  if ('Notification' in window && Notification.permission === 'default') {
    // 不立即请求权限，等用户触发时再请求
    console.log('通知权限未设置，将在需要时请求')
  }
}