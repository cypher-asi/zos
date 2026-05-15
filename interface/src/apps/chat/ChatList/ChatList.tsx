import { useCallback, useMemo } from "react";
import { useAppUIStore } from "../../../stores/app-ui-store";
import { useNavigate } from "../../../lib/router-adapter";
import { useChatStream } from "../../../shared/api/chatStream";
import { ChatConversationRow } from "../ChatConversationRow";
import { useSortedConversations } from "../stores/chat-store";
import { useConversationIdFromUrl } from "../useConversationParam";
import styles from "./ChatList.module.css";

export function ChatList() {
  // ChatList stays mounted for the full lifetime of the chat app, so
  // it is the natural owner of the long-lived WS that fans inbound
  // `MessageEnvelopeDto` frames into the react-query cache.
  useChatStream();

  const conversations = useSortedConversations();
  const selectedId = useConversationIdFromUrl();
  const navigate = useNavigate();
  const searchQuery = useAppUIStore((s) => s.sidebarQueries["chat"] ?? "");

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return conversations;
    const q = searchQuery.toLowerCase();
    return conversations.filter((c) => {
      const haystack = `${c.name} ${c.roleBadge ?? ""} ${c.lastSnippet}`.toLowerCase();
      return haystack.includes(q);
    });
  }, [conversations, searchQuery]);

  const handleSelect = useCallback(
    (id: string) => {
      navigate(`/chat/${id}`);
    },
    [navigate],
  );

  return (
    <div className={styles.sidebarRoot}>
      <div className={styles.sidebarScrollArea}>
        <div className={styles.sidebarEntries}>
          {filtered.length === 0 ? (
            <div className={styles.empty}>No conversations match your search.</div>
          ) : (
            filtered.map((conversation) => (
              <ChatConversationRow
                key={conversation.id}
                conversation={conversation}
                isSelected={conversation.id === selectedId}
                onClick={() => handleSelect(conversation.id)}
              />
            ))
          )}
        </div>
      </div>
    </div>
  );
}
