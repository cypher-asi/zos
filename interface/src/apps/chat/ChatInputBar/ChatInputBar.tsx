import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent,
} from "react";
import { SendHorizontal } from "lucide-react";
import { Button, Textarea } from "@cypher-asi/zui";
import { useChatStore } from "../stores/chat-store";
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
  const sendMessage = useChatStore((s) => s.sendMessage);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const id = requestAnimationFrame(() => textareaRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, []);

  const handleSend = useCallback(() => {
    const trimmed = draft.trim();
    if (!trimmed) return;
    sendMessage(conversationId, trimmed);
    setDraft("");
    requestAnimationFrame(() => textareaRef.current?.focus());
  }, [conversationId, draft, sendMessage]);

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

  return (
    <div className={styles.bar}>
      <div className={styles.inputWrap}>
        <Textarea
          ref={textareaRef}
          className={styles.textarea}
          placeholder="Message"
          value={draft}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          rows={1}
          aria-label="Message input"
        />
      </div>
      <Button
        variant="primary"
        iconOnly
        icon={<SendHorizontal size={16} />}
        onClick={handleSend}
        disabled={draft.trim().length === 0}
        aria-label="Send message"
        className={styles.sendButton}
      />
    </div>
  );
}
