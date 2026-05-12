import { create } from "zustand";
import type { Conversation, Message } from "../types";
import { SEED_CONVERSATIONS, SEED_MESSAGES } from "../data/seed";

interface ChatState {
  conversations: Conversation[];
  messagesByConversationId: Record<string, Message[]>;
  selectedId: string | null;
  selectConversation: (id: string | null) => void;
  sendMessage: (conversationId: string, content: string) => void;
}

function groupMessagesByConversation(messages: Message[]): Record<string, Message[]> {
  const acc: Record<string, Message[]> = {};
  for (const m of messages) {
    if (!acc[m.conversationId]) acc[m.conversationId] = [];
    acc[m.conversationId].push(m);
  }
  for (const id of Object.keys(acc)) {
    acc[id].sort((a, b) => a.createdAt.localeCompare(b.createdAt));
  }
  return acc;
}

export const useChatStore = create<ChatState>()((set) => ({
  conversations: SEED_CONVERSATIONS,
  messagesByConversationId: groupMessagesByConversation(SEED_MESSAGES),
  selectedId: null,
  selectConversation: (id) => set({ selectedId: id }),
  sendMessage: (conversationId, content) => {
    const trimmed = content.trim();
    if (!trimmed) return;
    set((s) => {
      const nowIso = new Date().toISOString();
      const message: Message = {
        id: `${conversationId}-${Date.now()}`,
        conversationId,
        role: "user",
        content: trimmed,
        createdAt: nowIso,
      };
      const existing = s.messagesByConversationId[conversationId] ?? [];
      const nextConversations = s.conversations.map((c) =>
        c.id === conversationId
          ? { ...c, lastSnippet: trimmed, updatedAt: nowIso }
          : c,
      );
      return {
        conversations: nextConversations,
        messagesByConversationId: {
          ...s.messagesByConversationId,
          [conversationId]: [...existing, message],
        },
      };
    });
  },
}));

export function useSortedConversations(): Conversation[] {
  return useChatStore((s) => {
    const list = [...s.conversations];
    list.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    return list;
  });
}

export function useConversation(id: string | null | undefined): Conversation | undefined {
  return useChatStore((s) =>
    id ? s.conversations.find((c) => c.id === id) : undefined,
  );
}

export function useMessages(id: string | null | undefined): Message[] {
  return useChatStore((s) => (id ? s.messagesByConversationId[id] ?? [] : []));
}
