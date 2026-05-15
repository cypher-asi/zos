//! `/api/chat/*` handlers — list/send DM messages, manage contacts,
//! and stream live message envelopes over a WebSocket.
//!
//! All read endpoints degrade gracefully to an empty list when no
//! identity exists yet (mirrors the runtime's
//! `IdentityMissing → empty` policy in `zos_grid::runtime`). Write
//! endpoints surface 4xx with the standard `ApiError` envelope.

use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, warn};

use zos_grid::{
    AddContactRequest, ContactDto, ConversationDto, CreateConversationRequest, GridFacadeError,
    ListConversationsQuery, ListMessagesQuery, MessageDto, MessageEnvelopeDto, SendMessageRequest,
};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Map a façade error onto the standard ApiError envelope. The chat
/// surface introduces a few new not-found / bad-input cases on top of
/// the original grid-mapper, so this lives next to the chat handlers
/// rather than re-using `handlers::grid::map_err`.
fn map_err(e: GridFacadeError) -> (StatusCode, Json<ApiError>) {
    match e {
        GridFacadeError::IdentityMissing => (
            StatusCode::CONFLICT,
            Json(ApiError {
                error: e.to_string(),
                code: Some("identity_missing".into()),
            }),
        ),
        GridFacadeError::BadHex(_) => (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: e.to_string(),
                code: Some("bad_hex".into()),
            }),
        ),
        GridFacadeError::NotFound(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: e.to_string(),
                code: Some("not_found".into()),
            }),
        ),
        other => ApiError::internal(other.to_string()),
    }
}

// ── REST handlers ─────────────────────────────────────────────────────────

pub(crate) async fn list_conversations(
    State(state): State<AppState>,
    Query(q): Query<ListConversationsQuery>,
) -> ApiResult<Json<Vec<ConversationDto>>> {
    let rows = state.grid.list_conversations(q.limit).await.map_err(map_err)?;
    Ok(Json(rows))
}

pub(crate) async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> ApiResult<(StatusCode, Json<ConversationDto>)> {
    let conv = state
        .grid
        .create_conversation(req.contact_id)
        .await
        .map_err(map_err)?;
    Ok((StatusCode::CREATED, Json(conv)))
}

pub(crate) async fn list_messages(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    Query(q): Query<ListMessagesQuery>,
) -> ApiResult<Json<Vec<MessageDto>>> {
    let rows = state
        .grid
        .list_messages(&conversation_id, q.before.as_deref(), q.limit)
        .await
        .map_err(map_err)?;
    Ok(Json(rows))
}

pub(crate) async fn send_message(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> ApiResult<(StatusCode, Json<MessageDto>)> {
    let trimmed = req.body.trim().to_string();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("message body must be non-empty"));
    }
    let dto = state
        .grid
        .send_message(&conversation_id, trimmed)
        .await
        .map_err(map_err)?;
    Ok((StatusCode::CREATED, Json(dto)))
}

pub(crate) async fn list_contacts(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ContactDto>>> {
    let rows = state.grid.list_contacts().await.map_err(map_err)?;
    Ok(Json(rows))
}

pub(crate) async fn add_contact(
    State(state): State<AppState>,
    Json(req): Json<AddContactRequest>,
) -> ApiResult<(StatusCode, Json<ContactDto>)> {
    let dto = state
        .grid
        .add_contact(req.label, req.identity_id)
        .await
        .map_err(map_err)?;
    Ok((StatusCode::CREATED, Json(dto)))
}

// ── WebSocket stream ──────────────────────────────────────────────────────

/// `GET /api/chat/stream` — upgrade to a WebSocket and pump every
/// [`MessageEnvelopeDto`] from the runtime's broadcast channel out to
/// the client as a JSON text frame.
///
/// The auth middleware already validated the JWT (via either the
/// `Authorization: Bearer ...` header or the `?token=...` query param —
/// see [`crate::auth_guard`]), so by the time the handler runs the
/// client is authenticated.
pub(crate) async fn chat_stream(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rx = state.grid.subscribe_messages();
    ws.on_upgrade(move |socket| stream_messages(socket, rx))
}

async fn stream_messages(
    socket: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<MessageEnvelopeDto>,
) {
    let (mut sink, mut stream) = socket.split();

    loop {
        tokio::select! {
            // Server → client: forward every new envelope.
            recv = rx.recv() => match recv {
                Ok(env) => {
                    let payload = match serde_json::to_string(&env) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!(error = %e, "chat ws: serialize envelope failed");
                            continue;
                        }
                    };
                    if let Err(e) = sink.send(WsMessage::Text(payload)).await {
                        debug!(error = %e, "chat ws: client closed (send)");
                        break;
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    // The slow consumer dropped frames. Tell the client
                    // to refetch from REST and continue streaming.
                    warn!(skipped, "chat ws: receiver lagged");
                    let notice = serde_json::json!({"event": "lagged", "skipped": skipped});
                    if let Err(e) = sink
                        .send(WsMessage::Text(notice.to_string()))
                        .await
                    {
                        debug!(error = %e, "chat ws: client closed (lagged)");
                        break;
                    }
                }
                Err(RecvError::Closed) => {
                    debug!("chat ws: broadcast channel closed");
                    break;
                }
            },
            // Client → server: drain control frames so the connection
            // can close cleanly. The client never sends application data
            // upstream over this channel.
            inbound = stream.next() => match inbound {
                Some(Ok(WsMessage::Close(_))) | None => {
                    debug!("chat ws: client requested close");
                    break;
                }
                Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_))) => {
                    // axum auto-handles ping/pong; nothing to do.
                }
                Some(Ok(WsMessage::Text(_) | WsMessage::Binary(_))) => {
                    // Ignore unsolicited application data.
                }
                Some(Err(e)) => {
                    debug!(error = %e, "chat ws: client read error");
                    break;
                }
            },
        }
    }
    let _ = sink.close().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{CachedSession, ValidationCache};
    use chrono::Utc;
    use dashmap::DashMap;
    use std::sync::Arc;
    use std::time::Instant;
    use tokio::sync::Mutex;
    use zos_auth::ZeroAuthSession;

    fn make_state(data_dir: std::path::PathBuf) -> AppState {
        let cache: ValidationCache = Arc::new(DashMap::new());
        let now = Utc::now();
        cache.insert(
            "jwt".into(),
            CachedSession {
                session: ZeroAuthSession {
                    user_id: "u".into(),
                    display_name: "T".into(),
                    profile_image: String::new(),
                    primary_zid: "0://t".into(),
                    zero_wallet: "0xabc".into(),
                    wallets: vec![],
                    access_token: "jwt".into(),
                    is_zero_pro: false,
                    created_at: now,
                    validated_at: now,
                },
                validated_at: Instant::now(),
            },
        );
        AppState {
            auth_service: Arc::new(zos_auth::AuthService::new()),
            validation_cache: cache,
            db: Arc::new(Mutex::new(vec![])),
            grid: zos_grid::ZeroRuntime::new(data_dir).unwrap(),
        }
    }

    #[tokio::test]
    async fn list_conversations_is_empty_without_identity() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let Json(rows) = list_conversations(
            State(state),
            Query(ListConversationsQuery::default()),
        )
        .await
        .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn list_messages_is_empty_without_identity() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let conv_id = "ab".repeat(32);
        let Json(rows) = list_messages(
            State(state),
            Path(conv_id),
            Query(ListMessagesQuery::default()),
        )
        .await
        .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn list_contacts_is_empty_without_identity() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let Json(rows) = list_contacts(State(state)).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn send_message_rejects_empty_body() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let conv_id = "ab".repeat(32);
        let err = send_message(
            State(state),
            Path(conv_id),
            Json(SendMessageRequest { body: "   ".into() }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_contact_without_identity_returns_409() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        let req = AddContactRequest {
            label: "Bob".into(),
            identity_id: "ab".repeat(16),
        };
        let err = add_contact(State(state), Json(req)).await.unwrap_err();
        assert_eq!(err.0, StatusCode::CONFLICT);
        assert_eq!(err.1.code.as_deref(), Some("identity_missing"));
    }

    #[tokio::test]
    async fn create_conversation_with_bad_hex_returns_400() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        // Need an identity for the runtime to reach the hex parser.
        state.grid.create_identity().unwrap();
        let req = CreateConversationRequest {
            contact_id: "not-hex".into(),
        };
        let err = create_conversation(State(state), Json(req))
            .await
            .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
        assert_eq!(err.1.code.as_deref(), Some("bad_hex"));
    }

    #[tokio::test]
    async fn add_then_list_contact_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(dir.path().to_path_buf());
        state.grid.create_identity().unwrap();
        let req = AddContactRequest {
            label: "Bob".into(),
            identity_id: "bb".repeat(16),
        };
        let (status, Json(created)) =
            add_contact(State(state.clone()), Json(req)).await.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created.label, "Bob");
        let Json(list) = list_contacts(State(state)).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "bb".repeat(16));
    }
}
