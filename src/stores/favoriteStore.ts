import { create } from 'zustand'
import { persist } from 'zustand/middleware'

interface FavoriteState {
  favorites: Set<string>

  // Actions
  addFavorite: (sessionId: string) => void
  removeFavorite: (sessionId: string) => void
  toggleFavorite: (sessionId: string) => void
  isFavorite: (sessionId: string) => boolean
}

export const useFavoriteStore = create<FavoriteState>()(
  persist(
    (set, get) => ({
      favorites: new Set<string>(),

      addFavorite: (sessionId: string) => {
        set((state) => {
          const newFavorites = new Set(state.favorites)
          newFavorites.add(sessionId)
          return { favorites: newFavorites }
        })
      },

      removeFavorite: (sessionId: string) => {
        set((state) => {
          const newFavorites = new Set(state.favorites)
          newFavorites.delete(sessionId)
          return { favorites: newFavorites }
        })
      },

      toggleFavorite: (sessionId: string) => {
        const state = get()
        if (state.favorites.has(sessionId)) {
          state.removeFavorite(sessionId)
        } else {
          state.addFavorite(sessionId)
        }
      },

      isFavorite: (sessionId: string) => {
        return get().favorites.has(sessionId)
      },
    }),
    {
      name: 'claude-fleet-favorites',
      // Set 需要特殊序列化
      storage: {
        getItem: (name) => {
          const str = localStorage.getItem(name)
          if (!str) return null
          const data = JSON.parse(str)
          return {
            ...data,
            state: {
              ...data.state,
              favorites: new Set(data.state.favorites || []),
            },
          }
        },
        setItem: (name, value) => {
          const data = {
            ...value,
            state: {
              ...value.state,
              favorites: Array.from(value.state.favorites),
            },
          }
          localStorage.setItem(name, JSON.stringify(data))
        },
        removeItem: (name) => localStorage.removeItem(name),
      },
    }
  )
)