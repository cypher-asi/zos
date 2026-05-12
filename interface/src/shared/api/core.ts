import type { ApiError } from "../types";
import { authHeaders } from "../lib/auth-token";

const BASE = "/api";

export class ApiClientError extends Error {
  status: number;
  body: ApiError;

  constructor(status: number, body: ApiError) {
    super(body.error);
    this.name = "ApiClientError";
    this.status = status;
    this.body = body;
  }
}

/**
 * Single canonical fetch helper for all `/api/...` calls. Handles JSON
 * encoding, JWT injection via `authHeaders()`, error envelope parsing into
 * `ApiClientError`, and the 204 No Content path.
 *
 * The `path` argument can be either a pre-prefixed `"/api/..."` string (the
 * convention used by aura-os, which we mirror here) or a bare `"..."`
 * suffix; the helper normalizes both so existing call sites keep working.
 */
export async function apiFetch<T>(
  path: string,
  options?: RequestInit,
): Promise<T> {
  const url = path.startsWith("/api") ? path : `${BASE}${path}`;
  const headers: Record<string, string> = { ...authHeaders() };
  const init: RequestInit = { ...options };
  if (options?.headers) {
    Object.assign(headers, options.headers as Record<string, string>);
  }
  if (init.body !== undefined && init.body !== null) {
    headers["Content-Type"] = headers["Content-Type"] ?? "application/json";
  }
  init.headers = headers;

  const res = await fetch(url, init);

  if (!res.ok) {
    const body: ApiError = await res.json().catch(() => ({
      error: res.statusText,
      code: "unknown",
    }));
    throw new ApiClientError(res.status, body);
  }

  if (res.status === 204) return undefined as T;
  return res.json();
}
