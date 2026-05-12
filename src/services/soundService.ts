import { invoke } from '@tauri-apps/api/core'
import type { SoundInfo } from '@/types'

// 内嵌默认音频（系统默认）
const BUILTIN_DEFAULT_SOUND = 'data:audio/wav;base64,UklGRnoGAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQoGAACBhYqFbF1fdH2Onp+dmZaRjo6OkZSYm56goaKjoqGgnpuYlZKQj4+QkpWYm56goaOjoqGgnpuYlZKQjo6OkZSYm52foKCgoJ+cmZWSkI6OjpGUl5ueoJ+fn56dmJWSkI6OjpGUl5udoKCfn56cmJaRjo6OkJOWl5udoJ+fnpyYlpCOjo6QkpWYm52fn5+enJiWkI6OjpCSlZibnJ6enpyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2dnJiWkI6OjpCSlZibnJ2dnZyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2dnJiWkI6OjpCSlZibnJ2dnZyYlpCOjo6QkpWYm5ydnZ2cmJaQjo6OkJKVmJucnZ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZ2cmJaQjo6OkJKVmJucnJ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZycmJaQjo6OkJKVmJucnJ2cnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZ2cmJaQjo6OkJKVmJucnJ2dnJiWkI6OjpCSlZibnJydnZyYlpCOjo6QkpWYm5ycnZycmJaQjo6OkJKVmJucnJycmJaQjo6OkJKVmJiXl5aVlJKSi4uLi4uLjJCRlZaYmp2enp2amJaUkY=='

// 内置音频标识
export const BUILTIN_DEFAULT_ID = 'builtin:default'

// 音频缓存
const soundCache = new Map<string, string>()
let availableSounds: SoundInfo[] | null = null

// 缓存内置音频
soundCache.set(BUILTIN_DEFAULT_ID, BUILTIN_DEFAULT_SOUND)

/**
 * 获取可用音频列表（包含内置默认）
 */
export async function getAvailableSounds(): Promise<SoundInfo[]> {
  if (availableSounds) {
    return availableSounds
  }

  try {
    const files = await invoke<{ name: string; filename: string }[]>('get_available_sounds')

    // 在最前面添加内置默认选项
    const builtinDefault: SoundInfo = {
      name: 'Default',
      filename: BUILTIN_DEFAULT_ID,
      isBuiltin: true,
    }

    // 其他文件
    const fileSounds: SoundInfo[] = files.map(f => ({
      name: f.name,
      filename: f.filename,
      isBuiltin: false,
    }))

    availableSounds = [builtinDefault, ...fileSounds]
    console.log('[soundService] 获取音频列表成功，共', availableSounds.length, '个')
    return availableSounds
  } catch (error) {
    console.error('[soundService] 获取音频列表失败:', error)
    // 只返回内置默认
    return [{ name: 'Default', filename: BUILTIN_DEFAULT_ID, isBuiltin: true }]
  }
}

/**
 * 获取音频数据（返回 base64 data URI）
 * @param filename 音频文件名（"builtin:default" 使用内置音频）
 * @param useFallback 失败时是否使用 fallback
 */
export async function getSoundData(filename: string, useFallback: boolean = true): Promise<string> {
  // 内置默认音频
  if (filename === BUILTIN_DEFAULT_ID || filename === '') {
    return BUILTIN_DEFAULT_SOUND
  }

  // 检查缓存
  if (soundCache.has(filename)) {
    return soundCache.get(filename)!
  }

  try {
    const dataUri = await invoke<string>('get_sound_data', { filename })
    soundCache.set(filename, dataUri)
    console.log('[soundService] 加载音频成功:', filename)
    return dataUri
  } catch (error) {
    console.error('[soundService] 加载音频失败:', filename, error)
    if (useFallback) {
      return BUILTIN_DEFAULT_SOUND
    }
    throw error
  }
}

/**
 * 试听音频
 * @param filename 音频文件名
 */
export async function previewSound(filename: string): Promise<void> {
  const soundData = await getSoundData(filename)
  const audio = new Audio(soundData)
  audio.volume = 0.5
  await audio.play()
}

/**
 * 清除音频缓存（保留内置默认）
 */
export function clearSoundCache(): void {
  soundCache.clear()
  soundCache.set(BUILTIN_DEFAULT_ID, BUILTIN_DEFAULT_SOUND)
  availableSounds = null
}