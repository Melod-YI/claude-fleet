import { create } from 'zustand'
import type { UpdateInfo } from '@/types'

interface UpdateState {
  updateInfo: UpdateInfo | null
  setUpdateInfo: (info: UpdateInfo | null) => void
}

export const useUpdateStore = create<UpdateState>()((set) => ({
  updateInfo: null,
  setUpdateInfo: (info) => set({ updateInfo: info }),
}))
