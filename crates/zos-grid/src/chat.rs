//! Chat-flavoured projection of the upstream `zero-sdk-10` messaging
//! services (`ContactStore`, `DmService`, `InboxService`) onto the wire
//! DTOs in [`crate::dto`].
//!
//! Phase-5 wiring pragmatics — what the SDK does and does not do today,
//! and how the helpers below paper over the gaps:
//!
//! * `DmService` exposes `send_text`, `history`, `get_message`, plus
//!   storage / status helpers — but **does not** itself update
//!   [`InboxService`] or notify subscribers about new messages.
//! * `InboxService` requires explicit `upsert` calls; nothing in the SDK
//!   pipeline currently calls it. The conversation list here is therefore
//!   derived primarily from [`ContactStore`] (one DM per contact) and only
//!   *enriched* with inbox metadata (unread count) when an entry happens
//!   to exist.
//! * The SDK's only inbound stream is [`zero_messaging::dm::DmReceiver`],
//!   which writes received DMs straight to RocksDB. There is no built-in
//!   pub/sub for "a new message just landed". Phase-5 therefore polls
//!   storage in a tokio task (see [`run_chat_poll_loop`]) and broadcasts
//!   [`MessageEnvelopeDto::Message`] frames via a `tokio::sync::broadcast`
//!   channel that the WebSocket handler subscribes to.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use tokio::sync::broadcast;
use tracing::{debug, trace, warn};

use zero_messaging::contacts::store::ContactStore;
use zero_messaging::contacts::types::{Contact, ContactMachineKey};
use zero_messaging::dm::{ConversationId, DmService, Message, MessageId, MessageStatus};
use zero_messaging::inbox::types::ConversationKind;
use zero_sdk::{IdentityId, ZeroSdk};

use crate::dto::{
    ContactDto, ContactMachineKeyDto, ConversationDto, MessageDto, MessageEnvelopeDto,
};
use crate::error::GridFacadeError;

/// Polling cadence for the inbound-message watcher. Two seconds keeps
/// "feels live" latency without dominating the CPU on an idle desktop.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Maximum number of messages we ever persist as "previously seen" per
/// conversation in the polling loop's snapshot. The loop only tracks the
/// id of the most-recent message, so this is intentionally `1`.
const POLL_PEEK_LIMIT: usize = 1;

// ── Hex parsers ───────────────────────────────────────────────────────────

/// Decode 64-char hex into a `ConversationId` (32 bytes).
pub(crate) fn parse_conversation_id(hex_str: &str) -> Result<ConversationId, GridFacadeError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| GridFacadeError::BadHex(format!("conversation_id: {e}")))?;
    let arr: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
        GridFacadeError::BadHex(format!("conversation_id: expected 32 bytes, got {}", v.len()))
    })?;
    Ok(ConversationId(arr))
}

/// Decode 32-char hex into a `MessageId` (16 bytes).
pub(crate) fn parse_message_id(hex_str: &str) -> Result<MessageId, GridFacadeError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| GridFacadeError::BadHex(format!("message_id: {e}")))?;
    let arr: [u8; 16] = bytes.try_into().map_err(|v: Vec<u8>| {
        GridFacadeError::BadHex(format!("message_id: expected 16 bytes, got {}", v.len()))
    })?;
    Ok(MessageId(arr))
}

/// Decode 32-char hex into an `IdentityId` (16 bytes).
pub(crate) fn parse_identity_id(hex_str: &str) -> Result<IdentityId, GridFacadeError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| GridFacadeError::BadHex(format!("identity_id: {e}")))?;
    let arr: [u8; 16] = bytes.try_into().map_err(|v: Vec<u8>| {
        GridFacadeError::BadHex(format!("identity_id: expected 16 bytes, got {}", v.len()))
    })?;
    Ok(IdentityId(arr))
}

// ── DTO converters ────────────────────────────────────────────────────────

fn status_str(s: MessageStatus) -> &'static str {
    match s {
        MessageStatus::Queued => "queued",
        MessageStatus::Sent => "sent",
        MessageStatus::Delivered => "delivered",
        MessageStatus::Read => "read",
    }
}

fn kind_str(k: ConversationKind) -> &'static str {
    match k {
        ConversationKind::Dm => "dm",
        ConversationKind::Group => "group",
    }
}

/// Build a wire `MessageDto` from an in-memory `Message`.
pub(crate) fn message_to_dto(msg: &Message) -> MessageDto {
    MessageDto {
        id: hex::encode(msg.id.0),
        conversation_id: hex::encode(msg.conversation_id.0),
        sender_machine_id: hex::encode(msg.sender_machine.0),
        sender_identity_id: hex::encode(msg.sender_identity.0),
        body: msg.text.clone(),
        sent_at: msg.created_at_ms,
        status: status_str(msg.status).to_string(),
    }
}

/// Build a wire `ContactDto` from an in-memory `Contact`.
pub(crate) fn contact_to_dto(c: &Contact) -> ContactDto {
    let id_hex = hex::encode(c.identity_id.0);
    ContactDto {
        id: id_hex.clone(),
        label: c.label.clone(),
        identity_id: id_hex,
        machine_keys: c.machine_keys.iter().map(machine_key_to_dto).collect(),
        added_at: c.added_at_ms,
    }
}

fn machine_key_to_dto(m: &ContactMachineKey) -> ContactMachineKeyDto {
    ContactMachineKeyDto {
        machine_id: hex::encode(m.machine_id.0),
        ed25519_pub_hex: hex::encode(m.ed25519_verifying),
        mldsa65_pub_hex: hex::encode(&m.mldsa_verifying),
    }
}

// ── Read paths ────────────────────────────────────────────────────────────

/// List the conversations visible to `sdk.identity_id`.
///
/// **Source-of-truth strategy** — because the upstream SDK does not yet
/// auto-populate the inbox, we synthesise one DM-shaped conversation row
/// per known contact, then *enrich* with inbox metadata when an entry
/// happens to exist. Conversations with at least one stored message are
/// sorted newest-first; contacts that have never exchanged a message yet
/// trail at the bottom alphabetically by label.
pub(crate) fn list_conversations(
    sdk: &ZeroSdk,
    limit: Option<usize>,
) -> Result<Vec<ConversationDto>, GridFacadeError> {
    let me = sdk.identity_id;
    let contacts = sdk.contacts.list_contacts()?;
    let inbox_entries = sdk.inbox.list_conversations(None)?;

    // Lookup table: ConversationId -> InboxEntry, so the per-contact loop
    // can do an O(1) inbox enrichment without re-scanning the inbox.
    let mut inbox_by_conv: HashMap<[u8; 32], &zero_messaging::inbox::InboxEntry> = HashMap::new();
    for entry in &inbox_entries {
        inbox_by_conv.insert(entry.conversation_id.0, entry);
    }

    let mut rows: Vec<ConversationDto> = Vec::with_capacity(contacts.len());
    for c in &contacts {
        let conv_id = ConversationId::derive(me, c.identity_id);
        let last = sdk.dm.history(conv_id, None, 1)?;
        let last_msg_at = last.first().map(|m| m.created_at_ms);
        let last_preview = last.first().map(|m| trim_preview(&m.text));

        let inbox = inbox_by_conv.get(&conv_id.0);
        let unread_count = inbox.map(|e| u64::from(e.unread)).unwrap_or(0);
        let last_message_at = inbox
            .map(|e| e.last_ts)
            .filter(|ts| *ts != 0)
            .or(last_msg_at);
        // Prefer the SDK-trimmed inbox preview when present (mirrors what
        // the inbox cap_preview already produced); fall back to whatever
        // we read from history.
        let last_message_preview = inbox
            .map(|e| e.preview.clone())
            .filter(|p| !p.is_empty())
            .or(last_preview);

        rows.push(ConversationDto {
            id: hex::encode(conv_id.0),
            kind: kind_str(ConversationKind::Dm).to_string(),
            contact_id: Some(hex::encode(c.identity_id.0)),
            name: Some(c.label.clone()),
            last_message_at,
            last_message_preview,
            unread_count,
        });
    }

    // Sort newest-first; conversations without messages sort last by label
    // for stable display.
    rows.sort_by(|a, b| match (b.last_message_at, a.last_message_at) {
        (Some(rb), Some(ra)) => rb.cmp(&ra),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a
            .name
            .as_deref()
            .unwrap_or("")
            .cmp(b.name.as_deref().unwrap_or("")),
    });

    if let Some(n) = limit {
        rows.truncate(n);
    }
    Ok(rows)
}

fn trim_preview(text: &str) -> String {
    const CAP: usize = 140;
    if text.chars().count() <= CAP {
        text.to_string()
    } else {
        text.chars().take(CAP).collect()
    }
}

/// Page through the stored history of `conversation_id`. Newest-first.
pub(crate) fn list_messages(
    sdk: &ZeroSdk,
    conversation_id_hex: &str,
    before: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<MessageDto>, GridFacadeError> {
    let conv_id = parse_conversation_id(conversation_id_hex)?;
    let before_id = match before {
        Some(s) if !s.is_empty() => Some(parse_message_id(s)?),
        _ => None,
    };
    let limit = limit.unwrap_or(50);
    let msgs = sdk.dm.history(conv_id, before_id, limit)?;
    Ok(msgs.iter().map(message_to_dto).collect())
}

/// List all contacts owned by `sdk.identity_id`.
pub(crate) fn list_contacts(sdk: &ZeroSdk) -> Result<Vec<ContactDto>, GridFacadeError> {
    let mut contacts = sdk.contacts.list_contacts()?;
    contacts.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(contacts.iter().map(contact_to_dto).collect())
}

// ── Write paths ───────────────────────────────────────────────────────────

/// Persist a new contact with the given label and identity id.
///
/// Idempotent on the underlying `ContactStore` (writing a second contact
/// for the same `IdentityId` overwrites the previous label) — see
/// `ContactStore::add_contact`'s "uniqueness on identity id" semantics.
pub(crate) fn add_contact(
    sdk: &ZeroSdk,
    label: String,
    identity_id_hex: String,
) -> Result<ContactDto, GridFacadeError> {
    let identity_id = parse_identity_id(&identity_id_hex)?;
    let added_at_ms = now_unix_ms();
    let contact = Contact {
        identity_id,
        label,
        machine_keys: Vec::new(),
        last_seen_epoch: None,
        added_at_ms,
    };
    sdk.contacts.add_contact(contact.clone())?;
    Ok(contact_to_dto(&contact))
}

/// Open or resume the DM conversation with `contact_id_hex`. Returns the
/// freshly synthesised [`ConversationDto`] (no messages yet ⇒
/// `last_message_at == None`).
pub(crate) fn create_conversation(
    sdk: &ZeroSdk,
    contact_id_hex: String,
) -> Result<ConversationDto, GridFacadeError> {
    let peer = parse_identity_id(&contact_id_hex)?;
    let _conv = sdk.dm.open_conversation(peer)?;

    // Reuse the same enrichment path the list endpoint uses so the
    // shape stays identical regardless of how the row was created.
    let me = sdk.identity_id;
    let conv_id = ConversationId::derive(me, peer);
    let label = sdk.contacts.get_contact(&peer)?.map(|c| c.label);
    let last = sdk.dm.history(conv_id, None, 1)?;
    let last_msg_at = last.first().map(|m| m.created_at_ms);
    let last_preview = last.first().map(|m| trim_preview(&m.text));

    Ok(ConversationDto {
        id: hex::encode(conv_id.0),
        kind: kind_str(ConversationKind::Dm).to_string(),
        contact_id: Some(hex::encode(peer.0)),
        name: label,
        last_message_at: last_msg_at,
        last_message_preview: last_preview,
        unread_count: 0,
    })
}

/// Send a plain-text message in `conversation_id_hex`, persisting it via
/// `DmService::send_text` and updating the inbox preview so subsequent
/// `list_conversations` calls see the new "last message" timestamp.
///
/// Returns the freshly-stored `MessageDto` (status: `Queued`). Callers
/// (the WS handler) are responsible for fanning the envelope out to
/// subscribers; this helper does **not** touch the broadcast channel so
/// the same code path can be reused from non-streaming contexts (tests).
pub(crate) fn send_message(
    sdk: &ZeroSdk,
    conversation_id_hex: &str,
    body: String,
) -> Result<MessageDto, GridFacadeError> {
    let conv_id = parse_conversation_id(conversation_id_hex)?;
    let msg_id = sdk.dm.send_text(conv_id, body.clone())?;
    let msg = sdk
        .dm
        .get_message(conv_id, msg_id)?
        .ok_or_else(|| GridFacadeError::NotFound(format!("message {msg_id} just sent")))?;

    // Best-effort inbox upsert so the conversation surfaces in the
    // inbox-driven preview list. Failure here should not fail the send —
    // the message is already persisted in the message index.
    let preview = zero_messaging::inbox::InboxEntry::cap_preview(&body);
    let entry = zero_messaging::inbox::InboxEntry {
        conversation_id: conv_id,
        kind: ConversationKind::Dm,
        last_ts: msg.created_at_ms,
        unread: 0,
        preview_sender: sdk.identity_id,
        preview,
    };
    if let Err(e) = sdk.inbox.upsert(entry) {
        warn!(error = %e, "chat: inbox upsert after send failed (non-fatal)");
    }

    Ok(message_to_dto(&msg))
}

// ── Polling task ──────────────────────────────────────────────────────────

/// Snapshot of "the most recent message id we have already broadcast" for
/// each conversation. Keyed by `ConversationId.0` so we don't pull a
/// `Hash` impl into scope.
type SeenMap = HashMap<[u8; 32], MessageId>;

/// Walk every contact's conversation, looking for messages we have not
/// already broadcast. Used by [`run_chat_poll_loop`].
fn poll_once(
    me: IdentityId,
    contacts: &Arc<ContactStore>,
    dm: &Arc<DmService>,
    seen: &mut SeenMap,
    seeded: bool,
    tx: &broadcast::Sender<MessageEnvelopeDto>,
) {
    let contact_list = match contacts.list_contacts() {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "chat poll: list_contacts failed");
            return;
        }
    };

    for c in &contact_list {
        let conv_id = ConversationId::derive(me, c.identity_id);
        // history(None, 1) returns the single newest message.
        let latest = match dm.history(conv_id, None, POLL_PEEK_LIMIT) {
            Ok(v) => v,
            Err(e) => {
                trace!(error = %e, conversation = %conv_id, "chat poll: history failed");
                continue;
            }
        };
        let Some(msg) = latest.into_iter().next() else {
            continue;
        };

        let prev = seen.insert(conv_id.0, msg.id);
        if !seeded {
            // First pass — record but don't broadcast historical state.
            continue;
        }
        if prev == Some(msg.id) {
            continue;
        }
        let envelope = MessageEnvelopeDto::Message {
            message: message_to_dto(&msg),
        };
        // Ignore `SendError`: it just means no subscribers are connected.
        let _ = tx.send(envelope);
    }
}

/// Detached background task: polls the local DB every [`POLL_INTERVAL`]
/// for newly-arrived messages and pushes them onto the runtime's chat
/// broadcast channel. Self-terminates when the parent runtime is
/// dropped (Weak upgrade fails).
pub(crate) async fn run_chat_poll_loop(
    rt_weak: Weak<crate::ZeroRuntime>,
    tx: broadcast::Sender<MessageEnvelopeDto>,
) {
    let mut seen: SeenMap = HashMap::new();
    let mut seeded = false;
    debug!("chat: poll loop started");

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let Some(rt) = rt_weak.upgrade() else {
            debug!("chat: runtime dropped, stopping poll loop");
            break;
        };
        let Some(sdk) = rt.chat_sdk_for_poll().await else {
            // No SDK opened yet (cold runtime, no identity, or DB
            // open failed) — nothing to poll, try again next tick.
            continue;
        };
        poll_once(
            sdk.identity_id,
            &sdk.contacts,
            &sdk.dm,
            &mut seen,
            seeded,
            &tx,
        );
        seeded = true;
    }
}

// ── Misc helpers ──────────────────────────────────────────────────────────

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zero_sdk::MachineId;

    #[test]
    fn parse_conversation_id_round_trip() {
        let raw = [0xABu8; 32];
        let hex = hex::encode(raw);
        let parsed = parse_conversation_id(&hex).unwrap();
        assert_eq!(parsed.0, raw);
    }

    #[test]
    fn parse_conversation_id_rejects_short_hex() {
        let err = parse_conversation_id("abcd").unwrap_err();
        assert!(matches!(err, GridFacadeError::BadHex(_)));
    }

    #[test]
    fn parse_conversation_id_rejects_non_hex() {
        let err = parse_conversation_id("zzzz").unwrap_err();
        assert!(matches!(err, GridFacadeError::BadHex(_)));
    }

    #[test]
    fn parse_message_id_round_trip() {
        let raw = [0x7Au8; 16];
        let hex = hex::encode(raw);
        assert_eq!(parse_message_id(&hex).unwrap().0, raw);
    }

    #[test]
    fn parse_identity_id_round_trip() {
        let raw = [0x42u8; 16];
        let hex = hex::encode(raw);
        assert_eq!(parse_identity_id(&hex).unwrap().0, raw);
    }

    #[test]
    fn message_to_dto_encodes_fields() {
        let msg = Message {
            id: MessageId([0x11; 16]),
            conversation_id: ConversationId([0x22; 32]),
            sender_identity: IdentityId([0x33; 16]),
            sender_machine: MachineId([0x44; 16]),
            text: "hi".into(),
            status: MessageStatus::Delivered,
            created_at_ms: 1_700_000_000_000,
            status_updated_at_ms: 1_700_000_000_000,
        };
        let dto = message_to_dto(&msg);
        assert_eq!(dto.id, "1".repeat(32));
        assert_eq!(dto.conversation_id, "2".repeat(64));
        assert_eq!(dto.sender_identity_id, "3".repeat(32));
        assert_eq!(dto.sender_machine_id, "4".repeat(32));
        assert_eq!(dto.body, "hi");
        assert_eq!(dto.status, "delivered");
        assert_eq!(dto.sent_at, 1_700_000_000_000);
    }

    #[test]
    fn contact_to_dto_round_trip() {
        let c = Contact {
            identity_id: IdentityId([0xAA; 16]),
            label: "Alice".into(),
            machine_keys: vec![],
            last_seen_epoch: None,
            added_at_ms: 42,
        };
        let dto = contact_to_dto(&c);
        assert_eq!(dto.id, "a".repeat(32));
        assert_eq!(dto.identity_id, dto.id);
        assert_eq!(dto.label, "Alice");
        assert_eq!(dto.added_at, 42);
        assert!(dto.machine_keys.is_empty());
    }

    #[test]
    fn message_envelope_serializes_with_event_tag() {
        let env = MessageEnvelopeDto::Message {
            message: MessageDto {
                id: "00".repeat(16),
                conversation_id: "00".repeat(32),
                sender_machine_id: "00".repeat(16),
                sender_identity_id: "00".repeat(16),
                body: "hi".into(),
                sent_at: 0,
                status: "queued".into(),
            },
        };
        let v = serde_json::to_value(&env).unwrap();
        assert_eq!(v["event"], "message");
        assert_eq!(v["message"]["body"], "hi");
    }
}
