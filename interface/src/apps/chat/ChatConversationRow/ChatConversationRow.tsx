import type { MouseEvent } from "react";
import { Avatar } from "@cypher-asi/zui";
import type { Conversation } from "../types";
import styles from "./ChatConversationRow.module.css";

interface ChatConversationRowProps {
  conversation: Conversation;
  isSelected: boolean;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}

function formatTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (24 * 60 * 60 * 1000));
  if (diffDays < 7) {
    return date.toLocaleDateString([], { weekday: "short" });
  }
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

export function ChatConversationRow({
  conversation,
  isSelected,
  onClick,
}: ChatConversationRowProps) {
  const className = isSelected ? `${styles.row} ${styles.selected}` : styles.row;

  return (
    <button
      type="button"
      id={conversation.id}
      className={className}
      onClick={onClick}
      aria-pressed={isSelected}
    >
      <Avatar
        name={conversation.name}
        src={conversation.avatarUrl}
        size="md"
        className={styles.avatar}
      />
      <span className={styles.body}>
        <span className={styles.top}>
          <span className={styles.name}>{conversation.name}</span>
          <span className={styles.time}>{formatTime(conversation.updatedAt)}</span>
        </span>
        <span className={styles.preview}>{conversation.lastSnippet}</span>
      </span>
    </button>
  );
}
