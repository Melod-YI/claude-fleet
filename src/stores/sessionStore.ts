import { create } from 'zustand'
import type { ClaudeSession, Conversation, SessionFilter } from '@/types'
import { listSessions, getConversation, refreshSessions } from '@/services'

interface SessionState {
  sessions: ClaudeSession[]
  selectedSessionId: string | null
  currentConversation: Conversation | null
  filter: SessionFilter
  loading: boolean
  error: string | null

  // Actions
  loadSessions: () => Promise<void>
  selectSession: (sessionId: string) => Promise<void>
  setFilter: (filter: Partial<SessionFilter>) => void
  refresh: () => Promise<void>
  clearError: () => void
}

export const useSessionStore = create<SessionState>((set) => ({
  sessions: [],
  selectedSessionId: null,
  currentConversation: null,
  filter: {
    showFavoritesOnly: true,
    timeRange: '30d',
  },
  loading: false,
  error: null,

  loadSessions: async () => {
    set({ loading: true, error: null })
    try {
      const sessions = await listSessions()
      set({ sessions, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  selectSession: async (sessionId: string) => {
    set({ selectedSessionId: sessionId, loading: true })
    try {
      const conversation = await getConversation(sessionId)
      set({ currentConversation: conversation, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  setFilter: (filter: Partial<SessionFilter>) => {
    set((state) => ({
      filter: { ...state.filter, ...filter }
    }))
  },

  refresh: async () => {
    set({ loading: true })
    try {
      const sessions = await refreshSessions()
      set({ sessions, loading: false })
    } catch (error) {
      set({ error: String(error), loading: false })
    }
  },

  clearError: () => set({ error: null }),
}))