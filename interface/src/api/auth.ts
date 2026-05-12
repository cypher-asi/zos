import type { AuthSession } from "../types/auth";
import { authHeaders } from "../lib/auth-token";

const BASE = "/api";

export class ApiClientError extends Error {
  status: number;
  body: { error: string; code?: string };

  constructor(status: number, body: { error: string; code?: string }) {
    super(body.error);
    this.name = "ApiClientError";
    this.status = status;
    this.body = body;
  }
}

async function authFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { "Content-Type": "application/json", ...authHeaders() },
    ...options,
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({
      error: res.statusText,
      code: "unknown",
    }));
    throw new ApiClientError(res.status, err);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const authApi = {
  login: (email: string, password: string) =>
    authFetch<AuthSession>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    }),

  register: (email: string, password: string, name: string, inviteCode: string) =>
    authFetch<AuthSession>("/auth/register", {
      method: "POST",
      body: JSON.stringify({ email, password, name, invite_code: inviteCode }),
    }),

  getSession: () => authFetch<AuthSession>("/auth/session"),

  validate: () =>
    authFetch<AuthSession>("/auth/validate", { method: "POST" }),

  logout: () =>
    authFetch<void>("/auth/logout", { method: "POST" }),

  requestPasswordReset: (email: string) =>
    authFetch<void>("/auth/request-password-reset", {
      method: "POST",
      body: JSON.stringify({ email }),
    }),
};
