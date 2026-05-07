import { useMemo } from "react";
import { useSessionsQuery, useDeleteSessionMutation } from "@/lib/query";
import { useSessionSearch } from "@/hooks/useSessionSearch";
import { useFavoriteStore } from "@/stores";

interface UseSessionsOptions {
  showFavoritesOnly?: boolean;
  searchQuery?: string;
}

export function useSessions(options?: UseSessionsOptions) {
  const { showFavoritesOnly = false, searchQuery = "" } = options ?? {};
  const { data, isLoading, error, refetch } = useSessionsQuery();
  const { favorites, isFavorite, toggleFavorite } = useFavoriteStore();
  const deleteMutation = useDeleteSessionMutation();

  const sessions = data ?? [];

  // Merge favorite status into sessions
  const sessionsWithFavorites = useMemo(() => {
    return sessions.map((session) => ({
      ...session,
      isFavorite: isFavorite(session.sessionId),
    }));
  }, [sessions, favorites]);

  // Use FlexSearch for full-text search
  const { search } = useSessionSearch({ sessions: sessionsWithFavorites });

  // Apply filters
  const filteredSessions = useMemo(() => {
    let result = search(searchQuery);

    if (showFavoritesOnly) {
      result = result.filter((s) => s.isFavorite);
    }

    return result;
  }, [search, searchQuery, showFavoritesOnly]);

  return {
    sessions: filteredSessions,
    allSessions: sessionsWithFavorites,
    loading: isLoading,
    error: error?.message ?? null,
    refresh: refetch,
    toggleFavorite,
    deleteSession: deleteMutation.mutate,
    isDeleting: deleteMutation.isPending,
  };
}