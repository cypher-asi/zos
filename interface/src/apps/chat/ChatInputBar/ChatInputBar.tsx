import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent,
} from "react";
import { ArrowUp, Plus } from "lucide-react";
import { Textarea } from "@cypher-asi/zui";
import { useSendMessageMutation } from "../stores/chat-data";
import styles from "./ChatInputBar.module.css";

interface ChatInputBarProps {
  conversationId: string;
}

/**
 * Reset on conversation switch is handled by the parent passing
 * `key={conversationId}`, so this component re-mounts with a clean
 * draft each time the active conversation changes.
 */
export function ChatInputBar({ conversationId }: ChatInputBarProps) {
  const [draft, setDraft] = useState("");
  const sendMutation = useSendMessageMutation();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const id = requestAnimationFrame(() => textareaRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, []);

  const handleSend = useCallback(() => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    sendMutation.mutate({ conversationId, body: trimmed });
    setDraft("");
    requestAnimationFrame(() => textareaRef.current?.focus());
  }, [conversationId, draft, sendMutation]);

  const handleChange = useCallback((event: ChangeEvent<HTMLTextAreaElement>) => {
    setDraft(event.target.value);
  }, []);

  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        handleSend();
      }
    },
    [handleSend],
  );

  const canSend = draft.trim().length > 0;

  return (
    <div className={styles.bar}>
      <div className={styles.composer}>
        <button
          type="button"
          className={styles.attachButton}
          aria-label="Add attachment"
        >
          <Plus size={14} />
        </button>
        <Textarea
          ref={textareaRef}
          className={styles.textarea}
          placeholder="Write a message…"
          value={draft}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          rows={1}
          aria-label="Message input"
        />
        <button
          type="button"
          className={styles.sendButton}
          onClick={handleSend}
          disabled={!canSend}
          aria-label="Send message"
        >
          <ArrowUp size={14} strokeWidth={2.5} />
        </button>
      </div>
    </div>
  );
}
