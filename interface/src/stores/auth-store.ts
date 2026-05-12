import { create } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { AuthSession, ZeroUser } from "../types/auth";
import {
  clearStoredAuth,
  getStoredSession,
  setStoredAuth,
} from "../lib/auth-token";
import { authApi, ApiClientError } from "../api/auth";

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

function isBypassActive(): boolean {
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
  restoreSession: () => Promise<void>;
  refreshSession: () => Promise<AuthSession>;
  login: (email: string, password: string) => Promise<void>;
  register: (
    email: string,
    password: string,
    name: string,
    inviteCode: string
  ) => Promise<void>;
  bypassLogin: () => void;
  logout: () => Promise<void>;
}

export const useAuthStore = create<AuthState>()((set) => ({
  user: null,
  isLoading: true,
  isBypass: false,

  restoreSession: async () => {
    if (isBypassActive()) {
      set({ user: BYPASS_USER, isBypass: true, isLoading: false });
      return;
    }

    const cached = getStoredSession();
    if (cached) {
      set({ user: sessionToUser(cached) });
    }

    try {
      const validated = await authApi.validate();
      setStoredAuth(validated);
      set({ user: sessionToUser(validated) });
    } catch (err) {
      if (err instanceof ApiClientError && err.status === 401) {
        clearStoredAuth();
        set({ user: null });
      }
      // Non-401 errors: keep cached session if available
    } finally {
      set({ isLoading: false });
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
      set({ isLoading: false });
    }
  },

  login: async (email: string, password: string) => {
    const session = await authApi.login(email, password);
    setStoredAuth(session);
    set({ user: sessionToUser(session) });
  },

  register: async (
    email: string,
    password: string,
    name: string,
    inviteCode: string
  ) => {
    const session = await authApi.register(email, password, name, inviteCode);
    setStoredAuth(session);
    set({ user: sessionToUser(session) });
  },

  bypassLogin: () => {
    setBypassActive(true);
    set({ user: BYPASS_USER, isBypass: true, isLoading: false });
  },

  logout: async () => {
    try {
      await authApi.logout();
    } catch {
      // ignore — we're tearing down the session anyway
    } finally {
      setBypassActive(false);
      clearStoredAuth();
      set({ user: null, isBypass: false });
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
      refreshSession: s.refreshSession,
      login: s.login,
      register: s.register,
      bypassLogin: s.bypassLogin,
      logout: s.logout,
    }))
  );
}
