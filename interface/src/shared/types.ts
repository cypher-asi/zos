export interface AuthSession {
  user_id: string;
  display_name: string;
  profile_image: string;
  primary_zid: string;
  zero_wallet: string;
  wallets: string[];
  is_zero_pro?: boolean;
  zero_pro_refresh_error?: string;
  access_token?: string;
  created_at: string;
  validated_at: string;
}

export interface ZeroUser {
  user_id: string;
  display_name: string;
  profile_image: string;
  primary_zid: string;
  zero_wallet: string;
  wallets: string[];
  is_zero_pro?: boolean;
}

export interface ApiError {
  error: string;
  code?: string;
}

// Grid / Identity / Devices DTOs.
//
// Field names mirror the snake_case JSON emitted by `crates/zos-grid/src/dto.rs`
// exactly. Hex-encoded IDs are 32-char ASCII strings (16 bytes).

export interface GridStatus {
  connected: boolean;
  multiaddr: string;
  /**
   * Custom connect-timeout in milliseconds, or `null` when the SDK
   * default (currently 30 000 ms) is in effect.
   */
  connect_timeout_ms: number | null;
  identity_id: string | null;
  last_error: string | null;
}

export interface IdentityDto {
  identity_id: string;
  epoch: number;
  created_at: number;
}

export interface DeviceDto {
  machine_id: string;
  identity_id: string;
  epoch: number;
  capabilities: number;
  created_at: number;
}

// `MachineKeyCapabilities` bitflags. Must stay in sync with the
// `bitflags!` definition in `zero_identity`.
export const Capability = {
  SEND: 0x01,
  RECEIVE: 0x02,
  MANAGE_MACHINES: 0x04,
  MANAGE_GROUPS: 0x08,
  ROTATE_EPOCH: 0x10,
} as const;

export type Capability = (typeof Capability)[keyof typeof Capability];

// Chat DTOs (mirror `crates/zos-grid/src/dto.rs`). All `*_id`/hex fields are
// ASCII hex; ConversationId is 64 chars (32 bytes), MessageId is 32 chars
// (16 bytes), IdentityId / MachineId are 32 chars (16 bytes).

export interface ConversationDto {
  id: string;
  kind: string;
  contact_id: string | null;
  name: string | null;
  last_message_at: number | null;
  last_message_preview: string | null;
  unread_count: number;
}

export interface MessageDto {
  id: string;
  conversation_id: string;
  sender_machine_id: string;
  sender_identity_id: string;
  body: string;
  sent_at: number;
  status: string;
}

export interface ContactMachineKeyDto {
  machine_id: string;
  ed25519_pub_hex: string;
  mldsa65_pub_hex: string;
}

export interface ContactDto {
  id: string;
  label: string;
  identity_id: string;
  machine_keys: ContactMachineKeyDto[];
  added_at: number;
}

/**
 * Discriminated union mirroring `MessageEnvelopeDto` on the server. We use a
 * narrow `event` tag so future variants (e.g. `conversation_updated`) can be
 * added without breaking existing handlers.
 */
export type ChatEnvelope = {
  event: "message";
  message: MessageDto;
};
