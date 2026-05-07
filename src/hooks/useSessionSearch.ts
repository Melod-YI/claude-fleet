import { useCallback, useMemo } from "react";
import FlexSearch from "flexsearch";
import type { SessionMeta } from "@/types";

interface UseSessionSearchOptions {
  sessions: SessionMeta[];
}

interface UseSessionSearchResult {
  search: (query: string) => SessionMeta[];
}

export function useSessionSearch({
  sessions,
}: UseSessionSearchOptions): UseSessionSearchResult {
  // Create FlexSearch index
  const index = useMemo(() => {
    const nextIndex = new FlexSearch.Index({
      tokenize: "full",
      resolution: 9,
    });

    sessions.forEach((session, idx) => {
      const metaContent = [
        session.sessionId,
        session.title,
        session.summary,
        session.projectDir,
        session.sourcePath,
      ]
        .filter(Boolean)
        .join(" ");

      nextIndex.add(idx, metaContent);
    });

    return nextIndex;
  }, [sessions]);

  // Search function
  const search = useCallback(
    (query: string): SessionMeta[] => {
      const needle = query.trim();

      if (!needle) {
        // No search query - return sorted by lastActiveAt
        return [...sessions].sort((a, b) => {
          const aTs = a.lastActiveAt ?? a.createdAt ?? 0;
          const bTs = b.lastActiveAt ?? b.createdAt ?? 0;
          return bTs - aTs;
        });
      }

      // FlexSearch results
      const results = index.search(needle, {
        limit: sessions.length,
      }) as number[];

      return results.map((idx) => sessions[idx]);
    },
    [index, sessions]
  );

  return { search };
}