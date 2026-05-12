import type { AuthSession } from "../types";

const JWT_STORAGE_KEY = "zero-jwt";
const SESSION_STORAGE_KEY = "zero-session";

// Legacy keys written by earlier ZERO builds. Read on first load only so a
// returning user with the old keys keeps their session, but never written
// going forward.
const LEGACY_JWT_STORAGE_KEY = "shell-jwt";
const LEGACY_SESSION_STORAGE_KEY = "shell-session";

function getLocalStorage(): Storage | null {
  if (typeof window === "undefined") return null;
  const storage = window.localStorage;
  return storage &&
    typeof storage.getItem === "function" &&
    typeof storage.setItem === "function" &&
    typeof storage.removeItem === "function"
    ? storage
    : null;
}

function normalizeSession(session: AuthSession | null): AuthSession | null {
  return session?.access_token ? session : null;
}

function parseStoredSession(
  raw: string | null,
  jwt: string | null,
): AuthSession | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as AuthSession;
    if (parsed?.access_token) return parsed;
    if (jwt) return normalizeSession({ ...parsed, access_token: jwt });
    return null;
  } catch {
    return null;
  }
}

function readSyncStoredSession(): AuthSession | null {
  const storage = getLocalStorage();
  if (!storage) return null;
  const jwt =
    storage.getItem(JWT_STORAGE_KEY) ?? storage.getItem(LEGACY_JWT_STORAGE_KEY);
  const raw =
    storage.getItem(SESSION_STORAGE_KEY) ??
    storage.getItem(LEGACY_SESSION_STORAGE_KEY);
  return parseStoredSession(raw, jwt);
}

function writeSyncStoredSession(session: AuthSession | null): void {
  const storage = getLocalStorage();
  if (!storage) return;
  if (session?.access_token) {
    storage.setItem(JWT_STORAGE_KEY, session.access_token);
    storage.setItem(SESSION_STORAGE_KEY, JSON.stringify(session));
  } else {
    storage.removeItem(JWT_STORAGE_KEY);
    storage.removeItem(SESSION_STORAGE_KEY);
  }
  // Always clear legacy keys; if a session is being written, the canonical
  // keys above are the source of truth from now on.
  storage.removeItem(LEGACY_JWT_STORAGE_KEY);
  storage.removeItem(LEGACY_SESSION_STORAGE_KEY);
}

/**
 * Module-level cache seeded once at import. Keeping it in memory means
 * `isLoggedInSync()`, `getStoredJwt()`, `getStoredSession()`, and
 * `authHeaders()` all read from the same coherent snapshot without paying
 * for repeated `localStorage.getItem` calls on the hot request path.
 */
let cachedSession: AuthSession | null = readSyncStoredSession();

/**
 * Synchronous "is the user logged in?" primitive used at app boot to seed
 * the auth store before any async restore work runs. Returning users get the
 * shell on the very first paint instead of a login flash.
 */
export function isLoggedInSync(): boolean {
  return Boolean(cachedSession?.access_token);
}

export function getStoredJwt(): string | null {
  return cachedSession?.access_token ?? null;
}

export function getStoredSession(): AuthSession | null {
  return cachedSession;
}

export function setStoredAuth(session: AuthSession | null): void {
  const normalized = normalizeSession(session);
  cachedSession = normalized;
  writeSyncStoredSession(normalized);
}

export function clearStoredAuth(): void {
  cachedSession = null;
  writeSyncStoredSession(null);
}

export function authHeaders(): Record<string, string> {
  const jwt = getStoredJwt();
  return jwt ? { Authorization: `Bearer ${jwt}` } : {};
}
