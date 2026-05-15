import type {
  ContactDto,
  ConversationDto,
  MessageDto,
} from "../types";
import { apiFetch } from "./core";

/**
 * Thin REST wrapper around the chat endpoints exposed by `zero-server`
 * (see `apps/zero-server/src/handlers/chat.rs`). Mirrors the per-method
 * shape of `gridApi`, `identityApi`, `devicesApi`.
 */
export const chatApi = {
  listConversations: (limit?: number): Promise<ConversationDto[]> => {
    const qs =
      limit !== undefined && Number.isFinite(limit)
        ? `?limit=${encodeURIComponent(String(limit))}`
        : "";
    return apiFetch<ConversationDto[]>(`/api/chat/conversations${qs}`);
  },

  createConversation: (contactId: string): Promise<ConversationDto> =>
    apiFetch<ConversationDto>("/api/chat/conversations", {
      method: "POST",
      body: JSON.stringify({ contact_id: contactId }),
    }),

  listMessages: (
    conversationId: string,
    opts?: { before?: string; limit?: number },
  ): Promise<MessageDto[]> => {
    const params = new URLSearchParams();
    if (opts?.before) params.set("before", opts.before);
    if (opts?.limit !== undefined && Number.isFinite(opts.limit)) {
      params.set("limit", String(opts.limit));
    }
    const qs = params.toString();
    const suffix = qs ? `?${qs}` : "";
    return apiFetch<MessageDto[]>(
      `/api/chat/conversations/${encodeURIComponent(conversationId)}/messages${suffix}`,
    );
  },

  sendMessage: (
    conversationId: string,
    body: string,
  ): Promise<MessageDto> =>
    apiFetch<MessageDto>(
      `/api/chat/conversations/${encodeURIComponent(conversationId)}/messages`,
      {
        method: "POST",
        body: JSON.stringify({ body }),
      },
    ),

  listContacts: (): Promise<ContactDto[]> =>
    apiFetch<ContactDto[]>("/api/chat/contacts"),

  addContact: (label: string, identityId: string): Promise<ContactDto> =>
    apiFetch<ContactDto>("/api/chat/contacts", {
      method: "POST",
      body: JSON.stringify({ label, identity_id: identityId }),
    }),
};
