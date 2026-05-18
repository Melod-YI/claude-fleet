import { create } from 'zustand'
import { addFavorite, removeFavorite, getAllFavorites } from '@/services/dbService'

interface FavoriteState {
  favorites: Set<string>
  initialized: boolean

  // Actions
  initialize: () => Promise<void>
  addFavorite: (sessionId: string) => Promise<void>
  removeFavorite: (sessionId: string) => Promise<void>
  toggleFavorite: (sessionId: string) => Promise<void>
  isFavorite: (sessionId: string) => boolean
}

export const useFavoriteStore = create<FavoriteState>()((set, get) => ({
  favorites: new Set<string>(),
  initialized: false,

  initialize: async () => {
    try {
      const favoriteIds = await getAllFavorites()
      set({ favorites: new Set(favoriteIds), initialized: true })
    } catch (e) {
      console.error('初始化收藏列表失败:', e)
      set({ favorites: new Set(), initialized: true })
    }
  },

  addFavorite: async (sessionId: string) => {
    await addFavorite(sessionId)
    set((state) => {
      const newFavorites = new Set(state.favorites)
      newFavorites.add(sessionId)
      return { favorites: newFavorites }
    })
  },

  removeFavorite: async (sessionId: string) => {
    await removeFavorite(sessionId)
    set((state) => {
      const newFavorites = new Set(state.favorites)
      newFavorites.delete(sessionId)
      return { favorites: newFavorites }
    })
  },

  toggleFavorite: async (sessionId: string) => {
    const state = get()
    if (state.favorites.has(sessionId)) {
      await state.removeFavorite(sessionId)
    } else {
      await state.addFavorite(sessionId)
    }
  },

  isFavorite: (sessionId: string) => {
    return get().favorites.has(sessionId)
  },
}))