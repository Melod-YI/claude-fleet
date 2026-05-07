import { invoke } from "@tauri-apps/api/core";
import type { SessionMeta, SessionMessage } from "@/types";

export const sessionsApi = {
  async list(): Promise<SessionMeta[]> {
    return await invoke("list_sessions_optimized");
  },

  async getMessages(sessionId: string): Promise<SessionMessage[]> {
    return await invoke("get_session_messages_optimized", { sessionId });
  },

  async delete(sessionId: string): Promise<boolean> {
    return await invoke("delete_session_optimized", { sessionId });
  },
};