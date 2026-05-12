import {
  isPermissionGranted,
  requestPermission,
  sendNotification as tauriSendNotification,
} from '@tauri-apps/plugin-notification'

// 内嵌 base64 提示音（简短的"叮"声 - 约 0.5 秒的合成音效）
// 这是一个有效的 WAV 格式音频，包含一个简单的正弦波提示音
const NOTIFICATION_SOUND_BASE64 =
  'data:audio/wav;base64,UklGRnoGAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQoGAACBhYqFbF1fdH2Onp+dmZaRjo6OkZSYm56goaKjoqGgnpuYlZKQj4+QkpWYm56goaOjoqGgnpuYlZKQjo6OkZSYm52foKCgoJ+cmZWSkI6OjpGUl5ueoJ+fn56dmJWSkI6OjpGUl5udoKCfn56cmJaRjo6OkJOWl5udoJ+fnpyYlpCOjo6QkpWYm52fn5+enJiWkI6OjpCSlZibnJ6enpyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2dnJiWkI6OjpCSlZibnJ2dnZyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2dnJiWkI6OjpCSlZibnJ2dnZyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2dnJiWkI6OjpCSlZibnJ2dnZyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2cnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZ2cmJaQjo6OkJKVmJucnJ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZ2cmJaQjo6OkJKVmJucnJ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZycmJaQjo6OkJKVmJucnJ2cnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZ2cmJaQjo6OkJKVmJucnJ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZycmJaQjo6OkJKVmJucnJ2cnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZ2cmJaQjo6OkJKVmJucnJ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZycmJaQjo6OkJKVmJucnJycmJaQjo6OkJKVmJiXl5aVlJKSi4uLi4uLjJCRlZaYmp2enp2amJaUkY=='

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
      notificationSound = new Audio(NOTIFICATION_SOUND_BASE64)
      notificationSound.volume = 0.5
      notificationSound.preload = 'auto'
      console.log('[notificationService] 音频初始化成功')
    } catch (e) {
      console.error('[notificationService] 初始化音频失败:', e)
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
      console.warn('[notificationService] 播放提示音失败:', e)
    })
  }
}

/**
 * 发送桌面通知
 */
export async function sendDesktopNotification(options: NotificationOptions): Promise<void> {
  console.log('[notificationService] 准备发送桌面通知:', options.title)

  try {
    // 检查通知权限
    let permissionGranted = await isPermissionGranted()
    console.log('[notificationService] 当前权限状态:', permissionGranted ? '已授权' : '未授权')

    if (!permissionGranted) {
      console.log('[notificationService] 请求通知权限...')
      const permission = await requestPermission()
      permissionGranted = permission === 'granted'
      console.log('[notificationService] 权限请求结果:', permission)
    }

    if (permissionGranted) {
      await tauriSendNotification({
        title: options.title,
        body: options.body,
      })
      console.log('[notificationService] 桌面通知已发送:', options.title)
    } else {
      console.warn('[notificationService] 通知权限未授权，无法发送通知')
    }
  } catch (e) {
    console.error('[notificationService] 发送桌面通知失败:', e)
  }

  // 播放提示音
  if (options.sound) {
    console.log('[notificationService] 播放提示音')
    playNotificationSound()
  }
}

/**
 * 初始化通知服务
 */
export async function initNotificationService(): Promise<void> {
  console.log('[notificationService] 开始初始化...')

  // 初始化音频
  initAudio()

  // 检查通知权限状态
  try {
    const permissionGranted = await isPermissionGranted()
    console.log('[notificationService] 初始化完成，权限状态:', permissionGranted ? '已授权' : '未授权')
  } catch (e) {
    console.error('[notificationService] 检查权限状态失败:', e)
  }
}