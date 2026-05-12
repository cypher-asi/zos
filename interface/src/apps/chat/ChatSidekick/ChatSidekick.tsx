import { useState, type ComponentType } from "react";
import {
  Ban,
  Bell,
  BellOff,
  FileText,
  Forward,
  Gift,
  Image as ImageIcon,
  Link2,
  MessageSquare,
  QrCode,
  Users,
  type LucideIcon,
} from "lucide-react";
import { Avatar, PageEmptyState, Text } from "@cypher-asi/zui";
import type { Conversation } from "../types";
import { useConversation } from "../stores/chat-store";
import { useConversationParam } from "../useConversationParam";
import styles from "./ChatSidekick.module.css";

interface ActionButtonProps {
  icon: LucideIcon;
  label: string;
  onClick?: () => void;
  active?: boolean;
}

function ActionButton({
  icon: Icon,
  label,
  onClick,
  active,
}: ActionButtonProps) {
  const className = active
    ? `${styles.actionButton} ${styles.actionButtonActive}`
    : styles.actionButton;
  return (
    <button type="button" className={className} onClick={onClick}>
      <Icon size={18} strokeWidth={1.75} />
      <span className={styles.actionLabel}>{label}</span>
    </button>
  );
}

interface InfoRowProps {
  value: string;
  label: string;
  trailing?: React.ReactNode;
}

function InfoRow({ value, label, trailing }: InfoRowProps) {
  return (
    <div className={styles.infoRow}>
      <div className={styles.infoText}>
        <span className={styles.infoValue}>{value}</span>
        <span className={styles.infoLabel}>{label}</span>
      </div>
      {trailing ? <div className={styles.infoTrailing}>{trailing}</div> : null}
    </div>
  );
}

interface CountRowProps {
  icon: LucideIcon;
  count: number;
  singular: string;
  plural: string;
}

function CountRow({ icon: Icon, count, singular, plural }: CountRowProps) {
  return (
    <button type="button" className={styles.countRow}>
      <Icon size={18} strokeWidth={1.75} className={styles.countIcon} />
      <span className={styles.countText}>
        {count} {count === 1 ? singular : plural}
      </span>
    </button>
  );
}

interface FooterActionProps {
  icon: LucideIcon;
  label: string;
  danger?: boolean;
  onClick?: () => void;
}

function FooterAction({ icon: Icon, label, danger, onClick }: FooterActionProps) {
  const className = danger
    ? `${styles.footerAction} ${styles.footerActionDanger}`
    : styles.footerAction;
  return (
    <button type="button" className={className} onClick={onClick}>
      <Icon size={18} strokeWidth={1.75} />
      <span>{label}</span>
    </button>
  );
}

interface ProfileViewProps {
  conversation: Conversation;
}

function ProfileView({ conversation }: ProfileViewProps) {
  const [muted, setMuted] = useState(false);

  return (
    <div className={styles.profile}>
      <section className={styles.identity}>
        <Avatar
          name={conversation.name}
          src={conversation.avatarUrl}
          size="xl"
          className={styles.avatar}
          style={{ width: 96, height: 96, fontSize: 32 }}
        />
        <div className={styles.nameLine}>
          <span className={styles.name}>{conversation.name}</span>
          {conversation.roleBadge ? (
            <span className={styles.roleBadge}>{conversation.roleBadge}</span>
          ) : null}
        </div>
        {conversation.lastSeen ? (
          <Text variant="muted" size="xs" className={styles.lastSeen}>
            last seen {conversation.lastSeen}
          </Text>
        ) : null}
      </section>

      <section className={styles.actionRow}>
        <ActionButton icon={MessageSquare} label="Message" />
        <ActionButton
          icon={muted ? Bell : BellOff}
          label={muted ? "Unmute" : "Mute"}
          active={muted}
          onClick={() => setMuted((prev) => !prev)}
        />
        <ActionButton icon={Gift} label="Gift" />
      </section>

      {(conversation.phone || conversation.bio || conversation.username) && (
        <section className={styles.infoSection}>
          {conversation.phone ? (
            <InfoRow
              value={conversation.phone}
              label={conversation.phoneLabel ?? "Mobile"}
            />
          ) : null}
          {conversation.bio ? (
            <InfoRow value={conversation.bio} label="Bio" />
          ) : null}
          {conversation.username ? (
            <InfoRow
              value={`@${conversation.username}`}
              label="Username"
              trailing={
                <button
                  type="button"
                  className={styles.qrButton}
                  aria-label="Show QR code"
                >
                  <QrCode size={16} strokeWidth={1.75} />
                </button>
              }
            />
          ) : null}
        </section>
      )}

      <section className={styles.contactsRow}>
        <button type="button" className={styles.addContactButton}>
          ADD TO CONTACTS
        </button>
      </section>

      <section className={styles.countsSection}>
        {conversation.photosCount != null ? (
          <CountRow
            icon={ImageIcon}
            count={conversation.photosCount}
            singular="photo"
            plural="photos"
          />
        ) : null}
        {conversation.filesCount != null ? (
          <CountRow
            icon={FileText}
            count={conversation.filesCount}
            singular="file"
            plural="files"
          />
        ) : null}
        {conversation.linksCount != null ? (
          <CountRow
            icon={Link2}
            count={conversation.linksCount}
            singular="shared link"
            plural="shared links"
          />
        ) : null}
        {conversation.groupsInCommonCount != null ? (
          <CountRow
            icon={Users}
            count={conversation.groupsInCommonCount}
            singular="group in common"
            plural="groups in common"
          />
        ) : null}
      </section>

      <section className={styles.footerSection}>
        <FooterAction icon={Forward} label="Share this contact" />
        <FooterAction icon={Ban} label="Block user" danger />
      </section>
    </div>
  );
}

export const ChatSidekick: ComponentType = () => {
  const conversationId = useConversationParam();
  const conversation = useConversation(conversationId);

  if (!conversationId || !conversation) {
    return (
      <PageEmptyState
        icon={<MessageSquare size={28} />}
        title="No conversation selected"
        description="Open a conversation to see contact details here."
      />
    );
  }

  return <ProfileView conversation={conversation} />;
};
