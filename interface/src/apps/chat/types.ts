export interface Conversation {
  id: string;
  name: string;
  roleBadge?: string;
  avatarUrl?: string;
  lastSnippet: string;
  updatedAt: string;
}

export type MessageRole = "user" | "contact";

export interface Message {
  id: string;
  conversationId: string;
  role: MessageRole;
  content: string;
  createdAt: string;
}
