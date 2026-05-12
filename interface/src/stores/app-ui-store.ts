import { create } from "zustand";
import type { ReactNode } from "react";
import { getPreviousPath, setPreviousPath } from "../utils/storage";

type AppUIState = {
  visitedAppIds: Set<string>;
  sidebarQueries: Record<string, string>;
  sidebarActions: Record<string, ReactNode>;
  sidekickCollapsed: boolean;
  appsModalOpen: boolean;
  backgroundModalOpen: boolean;
  previousPath: string | null;

  markAppVisited: (appId: string) => void;
  setSidebarQuery: (appId: string, query: string) => void;
  setSidebarAction: (appId: string, node: ReactNode | null) => void;
  toggleSidekick: () => void;
  openAppsModal: () => void;
  closeAppsModal: () => void;
  openBackgroundModal: () => void;
  closeBackgroundModal: () => void;
  setPreviousPath: (path: string) => void;
};

export const useAppUIStore = create<AppUIState>()((set) => ({
  visitedAppIds: new Set<string>(),
  sidebarQueries: {},
  sidebarActions: {},
  sidekickCollapsed: false,
  appsModalOpen: false,
  backgroundModalOpen: false,
  previousPath: typeof window === "undefined" ? null : getPreviousPath(),

  markAppVisited: (appId): void => {
    set((s) => {
      if (s.visitedAppIds.has(appId)) return s;
      const next = new Set(s.visitedAppIds);
      next.add(appId);
      return { visitedAppIds: next };
    });
  },

  setSidebarQuery: (appId, query): void => {
    set((s) => ({
      sidebarQueries: {
        ...s.sidebarQueries,
        [appId]: query,
      },
    }));
  },

  toggleSidekick: (): void => {
    set((s) => ({ sidekickCollapsed: !s.sidekickCollapsed }));
  },

  openAppsModal: (): void => set({ appsModalOpen: true }),
  closeAppsModal: (): void => set({ appsModalOpen: false }),
  openBackgroundModal: (): void => set({ backgroundModalOpen: true }),
  closeBackgroundModal: (): void => set({ backgroundModalOpen: false }),

  setPreviousPath: (path): void => {
    if (!path) return;
    setPreviousPath(path);
    set({ previousPath: path });
  },

  setSidebarAction: (appId, node): void => {
    set((s) => {
      if (node === null) {
        const nextSidebarActions = { ...s.sidebarActions };
        delete nextSidebarActions[appId];
        return { sidebarActions: nextSidebarActions };
      }
      return { sidebarActions: { ...s.sidebarActions, [appId]: node } };
    });
  },
}));
