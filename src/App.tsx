import { useState, useEffect } from "react"
import { invoke } from '@tauri-apps/api/core'
import { AppLayout } from "@/components/layout"
import { RunningTab } from "@/components/running"
import { ManagementTab } from "@/components/management"
import { useNotification } from "@/hooks"
import { ErrorBoundary } from "@/components/common"
import { useFavoriteStore, useSettingsStore } from '@/stores'
import { needsMigration, addFavorite, setSetting, recordPathUsage } from '@/services/dbService'

// 迁移 localStorage 数据到 SQLite
async function migrateFromLocalStorage() {
  // 迁移收藏
  const favoritesStr = localStorage.getItem('claude-fleet-favorites')
  if (favoritesStr) {
    try {
      const data = JSON.parse(favoritesStr)
      const favorites = data.state?.favorites || []
      for (const sessionId of favorites) {
        await addFavorite(sessionId)
      }
      localStorage.removeItem('claude-fleet-favorites')
      console.log('[Migration] 收藏迁移完成:', favorites.length)
    } catch (e) {
      console.error('[Migration] 迁移收藏失败:', e)
    }
  }

  // 迁移设置
  const settingsStr = localStorage.getItem('claude-fleet-settings')
  if (settingsStr) {
    try {
      const data = JSON.parse(settingsStr)
      const state = data.state || {}

      if (state.defaultTimeRange) {
        await setSetting('defaultTimeRange', state.defaultTimeRange)
      }
      if (state.notificationSound !== undefined) {
        await setSetting('notificationSound', state.notificationSound.toString())
      }
      if (state.notificationDesktop !== undefined) {
        await setSetting('notificationDesktop', state.notificationDesktop.toString())
      }
      if (state.notificationSoundFile) {
        await setSetting('notificationSoundFile', state.notificationSoundFile)
      }
      if (state.theme) {
        await setSetting('theme', state.theme)
      }
      if (state.terminalType) {
        await setSetting('terminalType', state.terminalType)
      }

      // 迁移常用路径
      const paths = state.favoritePaths?.paths || []
      for (const p of paths) {
        await recordPathUsage(p.path)
      }

      localStorage.removeItem('claude-fleet-settings')
      console.log('[Migration] 设置迁移完成')
    } catch (e) {
      console.error('[Migration] 迁移设置失败:', e)
    }
  }
}

function App() {
  const [activeTab, setActiveTab] = useState("running")
  const [isInitialized, setIsInitialized] = useState(false)

  // 使用通知 hook
  useNotification()

  // 初始化应用：迁移数据 + 加载 stores
  useEffect(() => {
    async function initializeApp() {
      try {
        // 检查是否需要迁移 localStorage 数据
        const shouldMigrate = await needsMigration()

        if (shouldMigrate) {
          console.log('[App] 开始迁移 localStorage 数据...')
          await migrateFromLocalStorage()
        }

        // 初始化 stores
        await useFavoriteStore.getState().initialize()
        await useSettingsStore.getState().initialize()

        setIsInitialized(true)
        console.log('[App] 初始化完成')
      } catch (e) {
        console.error('[App] 初始化失败:', e)
        setIsInitialized(true) // 即使失败也让应用继续运行
      }
    }

    initializeApp()
  }, [])

  // 启动 sessions 目录监听服务
  useEffect(() => {
    invoke('start_sessions_watcher').catch((e) => {
      console.error('启动 sessions 监听服务失败:', e)
    })

    return () => {
      invoke('stop_sessions_watcher').catch((e) => {
        console.error('停止 sessions 监听服务失败:', e)
      })
    }
  }, [])

  // 等待初始化完成
  if (!isInitialized) {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-muted-foreground">加载中...</div>
      </div>
    )
  }

  return (
    <ErrorBoundary>
      <AppLayout activeTab={activeTab} onTabChange={setActiveTab}>
        {activeTab === "running" && <RunningTab />}
        {activeTab === "management" && <ManagementTab />}
      </AppLayout>
    </ErrorBoundary>
  )
}

export default App