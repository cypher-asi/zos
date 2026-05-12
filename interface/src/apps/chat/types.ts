export interface Conversation {
  id: string;
  name: string;
  roleBadge?: string;
  avatarUrl?: string;
  lastSnippet: string;
  updatedAt: string;
  /** Display string for the contact's last-seen time, e.g. "recently". */
  lastSeen?: string;
  phone?: string;
  /** Label for the phone row, e.g. "Mobile". */
  phoneLabel?: string;
  bio?: string;
  /** Username without the leading @ sign. */
  username?: string;
  photosCount?: number;
  filesCount?: number;
  linksCount?: number;
  groupsInCommonCount?: number;
}

export type MessageRole = "user" | "contact";

export interface Message {
  id: string;
  conversationId: string;
  role: MessageRole;
  content: string;
  createdAt: string;
}
