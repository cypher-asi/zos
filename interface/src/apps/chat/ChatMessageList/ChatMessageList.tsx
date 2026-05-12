import { useEffect, useRef } from "react";
import type { Message } from "../types";
import styles from "./ChatMessageList.module.css";

interface ChatMessageListProps {
  messages: Message[];
}

function formatBubbleTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

export function ChatMessageList({ messages }: ChatMessageListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const node = scrollRef.current;
    if (!node) return;
    node.scrollTop = node.scrollHeight;
  }, [messages.length]);

  if (messages.length === 0) {
    return (
      <div ref={scrollRef} className={styles.scrollArea}>
        <div className={styles.empty}>No messages yet. Say hello.</div>
      </div>
    );
  }

  return (
    <div ref={scrollRef} className={styles.scrollArea}>
      <div className={styles.list}>
        {messages.map((message) => {
          const isUser = message.role === "user";
          const rowCls = `${styles.row} ${isUser ? styles.rowUser : styles.rowContact}`;
          const bubbleCls = `${styles.bubble} ${
            isUser ? styles.bubbleUser : styles.bubbleContact
          }`;
          return (
            <div key={message.id} className={rowCls}>
              <div>
                <div className={bubbleCls}>{message.content}</div>
                <div
                  className={styles.time}
                  style={{ textAlign: isUser ? "right" : "left" }}
                >
                  {formatBubbleTime(message.createdAt)}
                </div>
              </div>
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
