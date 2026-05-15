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
