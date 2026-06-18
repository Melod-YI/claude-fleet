import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { sessionsApi } from "@/lib/api/sessions";
import type { SessionMeta, SessionMessage } from "@/types";

export const useSessionsQuery = () => {
  return useQuery<SessionMeta[]>({
    queryKey: ["sessions"],
    queryFn: async () => sessionsApi.list(),
    staleTime: Infinity, // 不自动刷新，仅手动刷新
    gcTime: Infinity,    // 不自动回收缓存
    refetchOnWindowFocus: false, // 切换窗口时不自动刷新
  });
};

export const useSessionMessagesQuery = (
  sessionId?: string
): UseQueryResult<SessionMessage[]> => {
  return useQuery<SessionMessage[]>({
    queryKey: ["sessionMessages", sessionId],
    queryFn: async () => sessionsApi.getMessages(sessionId!),
    enabled: Boolean(sessionId),
    staleTime: 30 * 1000,
  });
};