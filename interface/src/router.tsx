import {
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
  Outlet,
} from "@tanstack/react-router";
import { Spinner } from "@cypher-asi/zui";
import { useAuthStore } from "./stores/auth-store";
import { Shell } from "./layout/Shell";
import { LoginView } from "./views/LoginView";

function PendingScreen() {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%" }}>
      <Spinner size="lg" />
    </div>
  );
}

function waitForAuth() {
  // We gate on `hasResolvedInitialSession` rather than `isLoading` because
  // the auth store seeds `user` synchronously from localStorage at module
  // import (see `seedAuthStateFromStorage` in stores/auth-store.ts). For a
  // returning user, that seed already lets the router make the right
  // decision on the very first navigation, and `restoreSession()` only
  // needs to flip this flag on completion to unblock new navigations after
  // a 401 has cleared the session.
  const state = useAuthStore.getState();
  if (state.hasResolvedInitialSession) return Promise.resolve(state);

  return new Promise<ReturnType<typeof useAuthStore.getState>>((resolve) => {
    const unsub = useAuthStore.subscribe((s) => {
      if (s.hasResolvedInitialSession) {
        unsub();
        resolve(s);
      }
    });
    const current = useAuthStore.getState();
    if (current.hasResolvedInitialSession) {
      unsub();
      resolve(current);
    }
  });
}

const rootRoute = createRootRoute({
  component: () => <Outlet />,
});

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "login",
  validateSearch: (search: Record<string, unknown>) => ({
    redirect: typeof search.redirect === "string" ? search.redirect : undefined,
  }),
  beforeLoad: async ({ search }) => {
    const state = await waitForAuth();
    if (state.user) {
      throw redirect({ to: search.redirect ?? "/" });
    }
  },
  component: LoginView,
});

const authRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "_authenticated",
  beforeLoad: async ({ location }) => {
    const state = await waitForAuth();
    if (!state.user) {
      throw redirect({
        to: "/login",
        search: { redirect: location.pathname },
      });
    }
  },
  pendingComponent: PendingScreen,
  component: () => <Outlet />,
});

const shellRoute = createRoute({
  getParentRoute: () => authRoute,
  id: "_shell",
  component: Shell,
});

const indexRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/chat" });
  },
});

const chatRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "chat",
  component: () => null,
});

const chatConversationRoute = createRoute({
  getParentRoute: () => chatRoute,
  path: "$conversationId",
  component: () => null,
});

const zeroRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "zero",
  component: () => null,
});

const projectsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "projects",
  component: () => null,
});

const projectDetailRoute = createRoute({
  getParentRoute: () => projectsRoute,
  path: "$projectId",
  component: () => null,
});

const exploreRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "explore",
  component: () => null,
});

const desktopRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "desktop",
  component: () => null,
});

const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "settings",
  component: () => null,
});

const routeTree = rootRoute.addChildren([
  loginRoute,
  authRoute.addChildren([
    shellRoute.addChildren([
      indexRoute,
      chatRoute.addChildren([chatConversationRoute]),
      zeroRoute,
      projectsRoute.addChildren([projectDetailRoute]),
      exploreRoute,
      settingsRoute,
      desktopRoute,
    ]),
  ]),
]);

export const router = createRouter({
  routeTree,
  defaultPendingComponent: PendingScreen,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
