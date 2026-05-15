import { useEffect, useRef } from "react";
import type { QueryClient } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import type { ChatEnvelope, MessageDto } from "../types";
import { getStoredJwt } from "../lib/auth-token";

const INITIAL_RECONNECT_MS = 1_000;
const MAX_RECONNECT_MS = 30_000;

/**
 * Resolve the WebSocket URL for `/api/chat/stream`, attaching the bearer
 * token as `?token=...` since browser WS APIs don't expose custom request
 * headers. The token is short-lived (validated on every connect) so we
 * accept the query-string surface here -- mirrors the path the auth_guard
 * already supports for WS in `apps/zero-server/src/auth_guard.rs`.
 */
function buildStreamUrl(): string | null {
  if (typeof window === "undefined") return null;
  const jwt = getStoredJwt();
  if (!jwt) return null;
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  const host = window.location.host;
  return `${proto}//${host}/api/chat/stream?token=${encodeURIComponent(jwt)}`;
}

/** Append a freshly-streamed message into the per-conversation cache. */
function applyMessageToCache(
  qc: QueryClient,
  message: MessageDto,
): void {
  const key = ["chat", "messages", message.conversation_id] as const;
  qc.setQueryData<MessageDto[] | undefined>(key, (prev) => {
    if (!prev) {
      qc.invalidateQueries({ queryKey: key });
      return prev;
    }
    if (prev.some((m) => m.id === message.id)) return prev;
    // history() returns newest-first; new pushes go on top.
    return [message, ...prev];
  });
  qc.invalidateQueries({ queryKey: ["chat", "conversations"] });
}

/**
 * Long-lived chat WebSocket. Owns the underlying `WebSocket` plus the
 * exponential-backoff reconnect loop and an `onmessage` dispatcher that
 * folds inbound `MessageEnvelopeDto` frames into the react-query cache.
 *
 * Designed to be created exactly once per chat-app mount via the
 * [`useChatStream`] hook below; manual instantiation is exported only
 * for tests and ad-hoc experiments.
 */
export class ChatStream {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectDelay = INITIAL_RECONNECT_MS;
  private closed = false;

  constructor(private readonly qc: QueryClient) {}

  start(): void {
    this.closed = false;
    this.connect();
  }

  /** Idempotent. After `stop()`, the instance can be discarded. */
  stop(): void {
    this.closed = true;
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      try {
        this.ws.close(1000, "client unmount");
      } catch {
        // best-effort
      }
      this.ws = null;
    }
  }

  private connect(): void {
    if (this.closed) return;
    const url = buildStreamUrl();
    if (!url) {
      // No JWT yet — defer; the hook reruns on auth changes.
      return;
    }

    let ws: WebSocket;
    try {
      ws = new WebSocket(url);
    } catch (e) {
      this.scheduleReconnect();
      return;
    }
    this.ws = ws;

    ws.onopen = () => {
      this.reconnectDelay = INITIAL_RECONNECT_MS;
    };

    ws.onmessage = (event) => {
      if (typeof event.data !== "string") return;
      let parsed: ChatEnvelope;
      try {
        parsed = JSON.parse(event.data) as ChatEnvelope;
      } catch {
        return;
      }
      if (parsed.event === "message" && parsed.message) {
        applyMessageToCache(this.qc, parsed.message);
      }
    };

    const handleClose = () => {
      this.ws = null;
      this.scheduleReconnect();
    };
    ws.onclose = handleClose;
    ws.onerror = handleClose;
  }

  private scheduleReconnect(): void {
    if (this.closed) return;
    const delay = this.reconnectDelay;
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, MAX_RECONNECT_MS);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, delay);
  }
}

/**
 * Mount-level hook that opens the chat WebSocket and fans inbound
 * `MessageEnvelopeDto` frames into the react-query cache. Call this once
 * near the top of the chat app (e.g. inside a component that's mounted
 * for the lifetime of the chat tab).
 */
export function useChatStream(): void {
  const qc = useQueryClient();
  const ref = useRef<ChatStream | null>(null);

  useEffect(() => {
    const stream = new ChatStream(qc);
    ref.current = stream;
    stream.start();
    return () => {
      stream.stop();
      ref.current = null;
    };
  }, [qc]);
}
