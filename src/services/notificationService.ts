import {
  isPermissionGranted,
  requestPermission,
  sendNotification as tauriSendNotification,
} from '@tauri-apps/plugin-notification'
import { getSoundData, BUILTIN_DEFAULT_ID } from './soundService'

// 音频状态
let currentSoundFile: string | null = null
let notificationSound: HTMLAudioElement | null = null

export interface NotificationOptions {
  title: string
  body: string
  sessionId?: string
  sound?: boolean
  soundFile?: string  // 指定音频文件（空或 "builtin:default" 使用内置）
}

/**
 * 初始化音频（使用指定或默认音频）
 */
async function initAudio(soundFile?: string): Promise<void> {
  try {
    // 确定使用哪个音频
    // 空字符串视为内置默认
    let targetFile = soundFile || currentSoundFile

    if (!targetFile) {
      targetFile = BUILTIN_DEFAULT_ID
    }

    // 加载音频数据
    const soundData = await getSoundData(targetFile)

    // 创建或更新音频元素
    notificationSound = new Audio(soundData)
    notificationSound.volume = 0.5
    notificationSound.preload = 'auto'

    currentSoundFile = targetFile
    console.log('[notificationService] 音频初始化成功:', targetFile)
  } catch (e) {
    console.error('[notificationService] 初始化音频失败:', e)
    // 使用内置默认
    const fallbackData = await getSoundData(BUILTIN_DEFAULT_ID)
    notificationSound = new Audio(fallbackData)
    notificationSound.volume = 0.5
  }
}

/**
 * 设置通知音频文件
 */
export async function setNotificationSoundFile(filename: string): Promise<boolean> {
  try {
    currentSoundFile = filename
    await initAudio(filename)
    return true
  } catch (e) {
    console.error('[notificationService] 设置音频失败:', e)
    return false
  }
}

/**
 * 播放通知提示音
 * @param soundFile 可选指定音频文件
 */
export async function playNotificationSound(soundFile?: string): Promise<void> {
  // 如果指定了不同的音频文件，重新初始化
  if (soundFile && soundFile !== currentSoundFile) {
    await initAudio(soundFile)
  } else if (!notificationSound) {
    await initAudio()
  }

  if (notificationSound) {
    notificationSound.currentTime = 0
    try {
      await notificationSound.play()
      console.log('[notificationService] 提示音播放成功')
    } catch (e) {
      console.warn('[notificationService] 播放提示音失败:', e)
    }
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
    await playNotificationSound(options.soundFile)
  }
}

/**
 * 初始化通知服务
 */
export async function initNotificationService(): Promise<void> {
  console.log('[notificationService] 开始初始化...')

  // 初始化音频
  await initAudio()

  // 检查通知权限状态
  try {
    const permissionGranted = await isPermissionGranted()
    console.log('[notificationService] 初始化完成，权限状态:', permissionGranted ? '已授权' : '未授权')
  } catch (e) {
    console.error('[notificationService] 检查权限状态失败:', e)
  }
}