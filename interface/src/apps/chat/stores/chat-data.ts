import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryOptions,
} from "@tanstack/react-query";
import { chatApi } from "../../../shared/api/chat";
import type {
  ContactDto,
  ConversationDto,
  MessageDto,
} from "../../../shared/types";

const CONVERSATIONS_KEY = ["chat", "conversations"] as const;
const CONTACTS_KEY = ["chat", "contacts"] as const;
const messagesKey = (conversationId: string | null | undefined) =>
  ["chat", "messages", conversationId ?? ""] as const;

/** List the current identity's known conversations. */
export function useConversationsQuery(
  opts?: Omit<UseQueryOptions<ConversationDto[]>, "queryKey" | "queryFn">,
) {
  return useQuery<ConversationDto[]>({
    queryKey: CONVERSATIONS_KEY,
    queryFn: () => chatApi.listConversations(),
    ...opts,
  });
}

/** Page through a single conversation's stored history (newest first). */
export function useMessagesQuery(
  conversationId: string | null | undefined,
  opts?: Omit<UseQueryOptions<MessageDto[]>, "queryKey" | "queryFn" | "enabled">,
) {
  return useQuery<MessageDto[]>({
    queryKey: messagesKey(conversationId),
    queryFn: () => chatApi.listMessages(conversationId!),
    enabled: Boolean(conversationId),
    ...opts,
  });
}

/** All known contacts owned by the current identity. */
export function useContactsQuery(
  opts?: Omit<UseQueryOptions<ContactDto[]>, "queryKey" | "queryFn">,
) {
  return useQuery<ContactDto[]>({
    queryKey: CONTACTS_KEY,
    queryFn: () => chatApi.listContacts(),
    ...opts,
  });
}

interface SendMessageVariables {
  conversationId: string;
  body: string;
}

/**
 * Persist a message via `POST /api/chat/conversations/:id/messages`,
 * with an optimistic local-echo that keeps the input bar feeling
 * snappy. The placeholder row is replaced atomically with the
 * server-confirmed `MessageDto` on success and rolled back on error.
 */
export function useSendMessageMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ conversationId, body }: SendMessageVariables) =>
      chatApi.sendMessage(conversationId, body),

    onMutate: async ({ conversationId, body }: SendMessageVariables) => {
      const key = messagesKey(conversationId);
      await qc.cancelQueries({ queryKey: key });
      const prev = qc.getQueryData<MessageDto[]>(key) ?? [];
      const optimisticId = `optimistic-${Date.now()}-${Math.random()
        .toString(36)
        .slice(2)}`;
      const optimistic: MessageDto = {
        id: optimisticId,
        conversation_id: conversationId,
        sender_machine_id: "",
        sender_identity_id: "",
        body,
        sent_at: Date.now(),
        status: "queued",
      };
      qc.setQueryData<MessageDto[]>(key, [optimistic, ...prev]);
      return { conversationId, optimisticId, prev };
    },

    onError: (_err, _vars, ctx) => {
      if (!ctx) return;
      qc.setQueryData<MessageDto[]>(
        messagesKey(ctx.conversationId),
        ctx.prev,
      );
    },

    onSuccess: (data, _vars, ctx) => {
      if (!ctx) return;
      const key = messagesKey(ctx.conversationId);
      qc.setQueryData<MessageDto[] | undefined>(key, (prev) => {
        if (!prev) return [data];
        // Drop the optimistic placeholder; if the server already
        // pushed via WS, dedupe on `id`.
        const without = prev.filter(
          (m) => m.id !== ctx.optimisticId && m.id !== data.id,
        );
        return [data, ...without];
      });
      qc.invalidateQueries({ queryKey: CONVERSATIONS_KEY });
    },
  });
}

/**
 * Open or resume the DM with `contactId` and surface it in the
 * conversation list.
 */
export function useCreateConversationMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (contactId: string) => chatApi.createConversation(contactId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: CONVERSATIONS_KEY });
    },
  });
}

interface AddContactVariables {
  label: string;
  identityId: string;
}

export function useAddContactMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ label, identityId }: AddContactVariables) =>
      chatApi.addContact(label, identityId),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: CONTACTS_KEY });
      qc.invalidateQueries({ queryKey: CONVERSATIONS_KEY });
    },
  });
}
