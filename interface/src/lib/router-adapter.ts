/**
 * Thin adapter that exposes a `react-router-dom`-style API
 * (`useNavigate(): (path: string) => void`, `useLocation(): { pathname }`)
 * on top of `@tanstack/react-router`.
 *
 * The aura-os components ported into this shell are written against
 * react-router-dom, so this adapter lets us drop them in unchanged
 * (`navigate("/foo")`, `useLocation().pathname`) while the underlying
 * router stays TanStack-Router.
 */
import {
  useLocation as useTanLocation,
  useNavigate as useTanNavigate,
} from "@tanstack/react-router";
import { useCallback, useMemo } from "react";

export interface ShellLocation {
  pathname: string;
  search: string;
  hash: string;
}

export function useLocation(): ShellLocation {
  const loc = useTanLocation();
  return useMemo(
    () => ({
      pathname: loc.pathname,
      search: typeof loc.search === "string" ? loc.search : "",
      hash: loc.hash ?? "",
    }),
    [loc.pathname, loc.search, loc.hash],
  );
}

export type NavigateFn = (
  to: string,
  options?: { replace?: boolean },
) => void;

export function useNavigate(): NavigateFn {
  const navigate = useTanNavigate();
  return useCallback<NavigateFn>(
    (to, options) => {
      navigate({ to, replace: options?.replace });
    },
    [navigate],
  );
}
