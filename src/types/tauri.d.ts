/// <reference types="@tauri-apps/api" />

interface TauriDialog {
  open: (options: {
    directory?: boolean
    multiple?: boolean
    defaultPath?: string
    filters?: Array<{
      name: string
      extensions: string[]
    }>
  }) => Promise<string | string[] | null>
}

interface TauriInvoke {
  invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>
}

declare global {
  interface Window {
    __TAURI__?: {
      dialog: TauriDialog
      invoke: TauriInvoke['invoke']
    }
  }
}

export {}