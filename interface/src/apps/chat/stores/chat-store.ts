import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { identityApi } from "../../../shared/api/identity";
import type { ConversationDto, MessageDto } from "../../../shared/types";
import type { Conversation, Message } from "../types";
import { SEED_CONVERSATIONS, SEED_MESSAGES } from "../data/seed";
import {
  useConversationsQuery,
  useMessagesQuery,
} from "./chat-data";

const EMPTY_MESSAGES: Message[] = [];
const EMPTY_CONVERSATIONS: Conversation[] = [];

const IDENTITY_KEY = ["identity"] as const;

/**
 * Per-mount toggle for keeping the seeded fixtures around when the
 * server has nothing to show. Helps the design review surface live
 * UI on a brand-new install (no identity, no contacts) without
 * reintroducing fake data into production once we ship.
 *
 * Wire `import.meta.env.VITE_CHAT_USE_SEED_FALLBACK = "1"` to enable.
 */
const SEED_FALLBACK = import.meta.env.VITE_CHAT_USE_SEED_FALLBACK === "1";

function fmtIsoFromMs(ms: number | null | undefined): string {
  if (ms == null) return "";
  return new Date(ms).toISOString();
}

/**
 * Map a server `ConversationDto` into the UI's local `Conversation`
 * shape. Many of the optional UI fields (`bio`, `username`,
 * `photosCount`, etc.) have no DTO counterpart yet and are simply left
 * undefined so the existing presentational components hide those
 * sections gracefully.
 */
function dtoToConversation(c: ConversationDto): Conversation {
  return {
    id: c.id,
    name: c.name ?? "Unknown",
    lastSnippet: c.last_message_preview ?? "",
    updatedAt: fmtIsoFromMs(c.last_message_at),
  };
}

function dtoToMessage(m: MessageDto, myIdentityHex: string | null): Message {
  const isUser = Boolean(
    myIdentityHex && m.sender_identity_id === myIdentityHex,
  );
  return {
    id: m.id,
    conversationId: m.conversation_id,
    role: isUser ? "user" : "contact",
    content: m.body,
    createdAt: fmtIsoFromMs(m.sent_at),
  };
}

/**
 * Conversation list, sorted newest-first to match the previous zustand
 * implementation. Falls back to the design seed when the API is empty
 * and `VITE_CHAT_USE_SEED_FALLBACK=1`.
 */
export function useSortedConversations(): Conversation[] {
  const query = useConversationsQuery();
  return useMemo(() => {
    const dtos = query.data;
    if (!dtos || dtos.length === 0) {
      return SEED_FALLBACK ? SEED_CONVERSATIONS : EMPTY_CONVERSATIONS;
    }
    const list = dtos.map(dtoToConversation);
    list.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    return list;
  }, [query.data]);
}

export function useConversation(
  id: string | null | undefined,
): Conversation | undefined {
  const list = useSortedConversations();
  return useMemo(
    () => (id ? list.find((c) => c.id === id) : undefined),
    [id, list],
  );
}

/**
 * Messages for `id`, oldest-first (the API returns newest-first; we
 * reverse on the client so the existing scroll-to-bottom behavior in
 * `ChatMessageList` keeps working unchanged).
 */
export function useMessages(id: string | null | undefined): Message[] {
  const messagesQuery = useMessagesQuery(id);
  const identityQuery = useQuery({
    queryKey: IDENTITY_KEY,
    queryFn: identityApi.get,
    staleTime: 5 * 60_000,
  });
  const myId = identityQuery.data?.identity_id ?? null;

  return useMemo(() => {
    if (!id) return EMPTY_MESSAGES;
    const dtos = messagesQuery.data;
    if (!dtos || dtos.length === 0) {
      if (SEED_FALLBACK) {
        return SEED_MESSAGES.filter((m) => m.conversationId === id);
      }
      return EMPTY_MESSAGES;
    }
    const mapped = dtos.map((m) => dtoToMessage(m, myId));
    mapped.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
    return mapped;
  }, [id, messagesQuery.data, myId]);
}
