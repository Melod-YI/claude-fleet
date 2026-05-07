import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { sessionsApi } from "@/lib/api/sessions";
import type { SessionMeta } from "@/types";

export const useDeleteSessionMutation = () => {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (sessionId: string) => {
      await sessionsApi.delete(sessionId);
      return sessionId;
    },
    onSuccess: async (sessionId) => {
      // Optimistically update cache - remove deleted session
      queryClient.setQueryData<SessionMeta[]>(["sessions"], (current) =>
        (current ?? []).filter((session) => session.sessionId !== sessionId)
      );

      // Remove cached messages for deleted session
      queryClient.removeQueries({
        queryKey: ["sessionMessages", sessionId],
      });

      // Trigger background refetch to ensure data consistency
      await queryClient.invalidateQueries({ queryKey: ["sessions"] });

      toast.success("会话已删除");
    },
    onError: (error: Error) => {
      toast.error(`删除会话失败: ${error.message}`);
    },
  });
};