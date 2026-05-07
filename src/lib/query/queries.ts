import { useQuery, type UseQueryResult } from "@tanstack/react-query";
import { sessionsApi } from "@/lib/api/sessions";
import type { SessionMeta, SessionMessage } from "@/types";

export const useSessionsQuery = () => {
  return useQuery<SessionMeta[]>({
    queryKey: ["sessions"],
    queryFn: async () => sessionsApi.list(),
    staleTime: 30 * 1000, // 30 seconds cache
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