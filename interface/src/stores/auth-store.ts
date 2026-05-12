import { create } from "zustand";
import { useShallow } from "zustand/react/shallow";
import type { AuthSession, ZeroUser } from "../types/auth";
import {
  clearStoredAuth,
  getStoredSession,
  setStoredAuth,
} from "../lib/auth-token";
import { authApi, ApiClientError } from "../api/auth";

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
  restoreSession: () => Promise<void>;
  refreshSession: () => Promise<AuthSession>;
  login: (email: string, password: string) => Promise<void>;
  register: (
    email: string,
    password: string,
    name: string,
    inviteCode: string
  ) => Promise<void>;
  logout: () => Promise<void>;
}

export const useAuthStore = create<AuthState>()((set) => ({
  user: null,
  isLoading: true,

  restoreSession: async () => {
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

  logout: async () => {
    try {
      await authApi.logout();
    } finally {
      clearStoredAuth();
      set({ user: null });
    }
  },
}));

export function useAuth() {
  return useAuthStore(
    useShallow((s) => ({
      user: s.user,
      isAuthenticated: s.user !== null,
      isLoading: s.isLoading,
      refreshSession: s.refreshSession,
      login: s.login,
      register: s.register,
      logout: s.logout,
    }))
  );
}
