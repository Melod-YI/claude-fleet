import { invoke } from '@tauri-apps/api/core'

// ========== Sessions Meta ==========

export async function setSessionName(sessionId: string, name: string): Promise<void> {
  await invoke('set_session_name_cmd', { sessionId, name })
}

export async function getSessionName(sessionId: string): Promise<string | null> {
  return await invoke('get_session_name_cmd', { sessionId })
}

export async function deleteSessionName(sessionId: string): Promise<void> {
  await invoke('delete_session_name_cmd', { sessionId })
}

// ========== Favorites ==========

export async function addFavorite(sessionId: string): Promise<void> {
  await invoke('add_favorite_cmd', { sessionId })
}

export async function removeFavorite(sessionId: string): Promise<void> {
  await invoke('remove_favorite_cmd', { sessionId })
}

export async function isFavorite(sessionId: string): Promise<boolean> {
  return await invoke('is_favorite_cmd', { sessionId })
}

export async function getAllFavorites(): Promise<string[]> {
  return await invoke('get_all_favorites_cmd')
}

// ========== Favorite Paths ==========

export async function recordPathUsage(path: string): Promise<void> {
  await invoke('record_path_usage_cmd', { path })
}

export async function removeFavoritePath(path: string): Promise<void> {
  await invoke('remove_favorite_path_cmd', { path })
}

export interface FavoritePath {
  path: string
  useCount: number
  lastUsedAt: number
  pinned: boolean
  pinnedAt: number | null
}

export async function getSortedFavoritePaths(): Promise<FavoritePath[]> {
  return await invoke('get_sorted_favorite_paths_cmd')
}

export async function togglePinPath(path: string): Promise<FavoritePath> {
  return await invoke('toggle_pin_path_cmd', { path })
}

// ========== Settings ==========

export async function getSetting(key: string): Promise<string | null> {
  return await invoke('get_setting_cmd', { key })
}

export async function setSetting(key: string, value: string): Promise<void> {
  await invoke('set_setting_cmd', { key, value })
}

export async function getAllSettings(): Promise<Record<string, string>> {
  return await invoke('get_all_settings_cmd')
}

export async function deleteSetting(key: string): Promise<void> {
  await invoke('delete_setting_cmd', { key })
}

// ========== Migration ==========

export async function needsMigration(): Promise<boolean> {
  return await invoke('needs_migration_cmd')
}