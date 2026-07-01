import { useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useUpdateStore } from '@/stores'
import type { UpdateInfo } from '@/types'

/**
 * 初始化更新检测：挂载时读取当前状态，并监听 update_available 事件。
 * 后端负责周期性请求 GitHub，前端只读。
 */
export function useUpdateChecker() {
  const setUpdateInfo = useUpdateStore((s) => s.setUpdateInfo)
  const unlistenRef = useRef<UnlistenFn | null>(null)

  useEffect(() => {
    // 读取后端当前状态（应用启动后可能已检测到）
    invoke<UpdateInfo | null>('get_update_status')
      .then((info) => setUpdateInfo(info))
      .catch((e) => console.error('[useUpdateChecker] 读取更新状态失败:', e))

    // 监听新版本事件
    const setup = async () => {
      unlistenRef.current = await listen<UpdateInfo>('update_available', (event) => {
        setUpdateInfo(event.payload)
      })
    }
    setup()

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current()
      }
    }
  }, [setUpdateInfo])
}
