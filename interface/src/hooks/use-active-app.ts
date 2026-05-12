import { useMemo } from "react";
import type { ShellApp } from "../shell/types";
import { resolveActiveApp } from "../stores/app-store";
import { useLocation } from "../lib/router-adapter";

/**
 * Resolves the currently-active app from the router's pathname. Computing
 * this synchronously from `useLocation()` keeps the shell chrome in lockstep
 * with the URL — there is no `activeApp` store field that could lag a
 * `navigate()` call by one render.
 */
export function useActiveApp(): ShellApp {
  const { pathname } = useLocation();
  return useMemo(() => resolveActiveApp(pathname), [pathname]);
}

export function useActiveAppId(): string {
  return useActiveApp().id;
}
