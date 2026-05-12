import { create } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { AuthSession, ZeroUser } from "../shared/types";
import {
  clearStoredAuth,
  getStoredJwt,
  getStoredSession,
  isLoggedInSync,
  setStoredAuth,
} from "../shared/lib/auth-token";
import { authApi } from "../shared/api/auth";
import { ApiClientError } from "../shared/api/core";

const BYPASS_STORAGE_KEY = "zero-dev-bypass";

const BYPASS_USER: ZeroUser = {
  user_id: "dev-bypass",
  display_name: "Dev User",
  profile_image: "",
  primary_zid: "0://dev",
  zero_wallet: "",
  wallets: [],
  is_zero_pro: false,
};

function isDevBuild(): boolean {
  return Boolean(import.meta.env.DEV);
}

function isBypassActive(): boolean {
  if (!isDevBuild()) return false;
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(BYPASS_STORAGE_KEY) === "1";
}

function setBypassActive(active: boolean): void {
  if (typeof window === "undefined") return;
  if (active) window.localStorage.setItem(BYPASS_STORAGE_KEY, "1");
  else window.localStorage.removeItem(BYPASS_STORAGE_KEY);
}

function sessionToUser(session: AuthSession): ZeroUser {
  return {
    user_id: session.user_id,
    display_name: session.display_name,
    profile_image: session.profile_image,
    primary_zid: session.primary_zid,
    zero_wallet: session.zero_wallet,
    wallets: session.wallets,
    is_zero_pro: session.is_zero_pro,
  };
}

interface AuthState {
  user: ZeroUser | null;
  isLoading: boolean;
  isBypass: boolean;
  /**
   * Flips `true` exactly once, after the first boot-time `restoreSession()`
   * (or any login/register/logout) finishes. The router uses this — not
   * `isLoading` — as the boot gate so a returning user with a cached session
   * never sees a spinner: the store seeds `user` synchronously from
   * localStorage in `seedAuthStateFromStorage()`, and the gate flips on the
   * very first `set` from `restoreSession`.
   */
  hasResolvedInitialSession: boolean;
  restoreSession: () => Promise<void>;
  refreshSession: () => Promise<AuthSession>;
  login: (email: string, password: string) => Promise<void>;
  register: (
    email: string,
    password: string,
    name: string,
    inviteCode: string,
  ) => Promise<void>;
  bypassLogin: () => void;
  logout: () => Promise<void>;
}

/**
 * Seed the auth store synchronously at module import using the same
 * `isLoggedInSync()` primitive the router checks at boot. This guarantees
 * the very first React paint already has the right `user` value for
 * returning users — no login flash.
 */
function seedAuthStateFromStorage(): Pick<
  AuthState,
  "user" | "isLoading" | "isBypass" | "hasResolvedInitialSession"
> {
  if (isBypassActive()) {
    return {
      user: BYPASS_USER,
      isLoading: false,
      isBypass: true,
      hasResolvedInitialSession: true,
    };
  }
  if (isLoggedInSync()) {
    const cached = getStoredSession();
    if (cached) {
      return {
        user: sessionToUser(cached),
        isLoading: false,
        isBypass: false,
        hasResolvedInitialSession: false,
      };
    }
  }
  return {
    user: null,
    isLoading: true,
    isBypass: false,
    hasResolvedInitialSession: false,
  };
}

export const useAuthStore = create<AuthState>()((set) => ({
  ...seedAuthStateFromStorage(),

  restoreSession: async () => {
    if (isBypassActive()) {
      set({
        user: BYPASS_USER,
        isBypass: true,
        isLoading: false,
        hasResolvedInitialSession: true,
      });
      return;
    }

    // No JWT means there's nothing to restore — short-circuit so we don't
    // burn a guaranteed-401 round-trip on every cold open.
    if (!getStoredJwt()) {
      set({
        user: null,
        isLoading: false,
        hasResolvedInitialSession: true,
      });
      return;
    }

    try {
      // GET /api/auth/session uses the server's TTL cache for the second
      // and later boots within the cache window, so this is cheap.
      const validated = await authApi.getSession();
      setStoredAuth(validated);
      set({ user: sessionToUser(validated) });
    } catch (err) {
      if (err instanceof ApiClientError && err.status === 401) {
        clearStoredAuth();
        set({ user: null });
      }
      // Non-401 (e.g. server unreachable): keep the seeded cached session.
    } finally {
      set({ isLoading: false, hasResolvedInitialSession: true });
    }
  },

  refreshSession: async () => {
    set({ isLoading: true });
    try {
      const validated = await authApi.validate();
      setStoredAuth(validated);
      set({ user: sessionToUser(validated) });
      return validated;
    } catch (err) {
      if (err instanceof ApiClientError && err.status === 401) {
        clearStoredAuth();
        set({ user: null });
      }
      throw err;
    } finally {
      set({ isLoading: false, hasResolvedInitialSession: true });
    }
  },

  login: async (email: string, password: string) => {
    const session = await authApi.login(email, password);
    setStoredAuth(session);
    setBypassActive(false);
    set({
      user: sessionToUser(session),
      isBypass: false,
      hasResolvedInitialSession: true,
    });
  },

  register: async (
    email: string,
    password: string,
    name: string,
    inviteCode: string,
  ) => {
    const session = await authApi.register(email, password, name, inviteCode);
    setStoredAuth(session);
    setBypassActive(false);
    set({
      user: sessionToUser(session),
      isBypass: false,
      hasResolvedInitialSession: true,
    });
  },

  bypassLogin: () => {
    if (!isDevBuild()) return;
    setBypassActive(true);
    set({
      user: BYPASS_USER,
      isBypass: true,
      isLoading: false,
      hasResolvedInitialSession: true,
    });
  },

  logout: async () => {
    try {
      await authApi.logout();
    } catch {
      // ignore — we're tearing down the session anyway
    } finally {
      setBypassActive(false);
      clearStoredAuth();
      set({
        user: null,
        isBypass: false,
        hasResolvedInitialSession: true,
      });
    }
  },
}));

export function useAuth() {
  return useAuthStore(
    useShallow((s) => ({
      user: s.user,
      isAuthenticated: s.user !== null,
      isLoading: s.isLoading,
      isBypass: s.isBypass,
      hasResolvedInitialSession: s.hasResolvedInitialSession,
      refreshSession: s.refreshSession,
      login: s.login,
      register: s.register,
      bypassLogin: s.bypassLogin,
      logout: s.logout,
    })),
  );
}

export const isDevBypassAvailable = isDevBuild;
