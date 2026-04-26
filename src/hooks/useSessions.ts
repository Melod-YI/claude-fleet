import { useEffect, useMemo } from 'react'
import { useSessionStore, useFavoriteStore } from '@/stores'
import { searchSessions, filterByTimeRange } from '@/utils'

export function useSessions() {
  const { sessions, filter, loading, error, loadSessions, setFilter } = useSessionStore()
  const { favorites, isFavorite, toggleFavorite } = useFavoriteStore()

  // 初始加载
  useEffect(() => {
    loadSessions()
  }, [loadSessions])

  // 合合收藏状态到 session
  const sessionsWithFavorites = useMemo(() => {
    return sessions.map((session) => ({
      ...session,
      isFavorite: isFavorite(session.id),
    }))
  }, [sessions, favorites])

  // 应用过滤条件
  const filteredSessions = useMemo(() => {
    let result = sessionsWithFavorites

    // 收藏过滤
    if (filter.showFavoritesOnly) {
      result = result.filter((s) => s.isFavorite)
    }

    // 时间过滤（仅在非收藏模式时应用）
    if (!filter.showFavoritesOnly && filter.timeRange) {
      result = filterByTimeRange(result, filter.timeRange)
    }

    // 搜索过滤
    if (filter.searchQuery) {
      result = searchSessions(result, filter.searchQuery)
    }

    return result
  }, [sessionsWithFavorites, filter])

  return {
    sessions: filteredSessions,
    allSessions: sessionsWithFavorites,
    loading,
    error,
    filter,
    setFilter,
    toggleFavorite,
    refresh: loadSessions,
  }
}