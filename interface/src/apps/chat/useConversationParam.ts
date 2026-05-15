import { useMemo } from "react";
import { useLocation } from "../../lib/router-adapter";

const CHAT_PATH_PATTERN = /^\/chat\/([^/]+)/;

export function useConversationIdFromUrl(): string | null {
  const { pathname } = useLocation();
  return useMemo(() => {
    const match = CHAT_PATH_PATTERN.exec(pathname);
    return match ? decodeURIComponent(match[1]) : null;
  }, [pathname]);
}

/**
 * Single source of truth for "which conversation is the chat app
 * showing right now". The URL is authoritative -- the previous
 * zustand-backed `selectConversation` mirror is intentionally gone now
 * that all conversation data flows through react-query.
 */
export function useConversationParam(): string | null {
  return useConversationIdFromUrl();
}
