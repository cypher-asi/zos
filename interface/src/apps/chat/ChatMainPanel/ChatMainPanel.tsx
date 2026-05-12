import { MessageSquare } from "lucide-react";
import { Avatar, PageEmptyState } from "@cypher-asi/zui";
import { ChatInputBar } from "../ChatInputBar";
import { ChatMessageList } from "../ChatMessageList";
import { useConversation, useMessages } from "../stores/chat-store";
import { useConversationParam } from "../useConversationParam";
import styles from "./ChatMainPanel.module.css";

export function ChatMainPanel() {
  const conversationId = useConversationParam();
  const conversation = useConversation(conversationId);
  const messages = useMessages(conversationId);

  if (!conversationId || !conversation) {
    return (
      <PageEmptyState
        icon={<MessageSquare size={32} />}
        title="Select a conversation"
        description="Pick a conversation from the sidebar to view messages and reply."
      />
    );
  }

  return (
    <div className={styles.container}>
      <header className={styles.header}>
        <Avatar
          name={conversation.name}
          src={conversation.avatarUrl}
          size="md"
        />
        <div className={styles.headerText}>
          <span className={styles.headerName}>
            {conversation.name}
            {conversation.roleBadge && (
              <span className={styles.roleBadge}>{conversation.roleBadge}</span>
            )}
          </span>
        </div>
      </header>
      <div className={styles.body}>
        <ChatMessageList messages={messages} />
        <ChatInputBar key={conversationId} conversationId={conversationId} />
      </div>
    </div>
  );
}
