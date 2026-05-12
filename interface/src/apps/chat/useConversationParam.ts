import { useEffect, useMemo } from "react";
import { useLocation } from "../../lib/router-adapter";
import { useChatStore } from "./stores/chat-store";

const CHAT_PATH_PATTERN = /^\/chat\/([^/]+)/;

export function useConversationIdFromUrl(): string | null {
  const { pathname } = useLocation();
  return useMemo(() => {
    const match = CHAT_PATH_PATTERN.exec(pathname);
    return match ? decodeURIComponent(match[1]) : null;
  }, [pathname]);
}

export function useConversationParam(): string | null {
  const urlId = useConversationIdFromUrl();
  const selectConversation = useChatStore((s) => s.selectConversation);

  useEffect(() => {
    selectConversation(urlId);
  }, [urlId, selectConversation]);

  return urlId;
}
