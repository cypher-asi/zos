import { create } from "zustand";
import type { ShellApp } from "../shell/types";
import { apps as registeredApps } from "../shell/registry";
import {
  getTaskbarAppOrder,
  getTaskbarHiddenAppIds,
  setTaskbarAppOrder,
  setTaskbarHiddenAppIds,
} from "../utils/storage";

interface AppState {
  apps: ShellApp[];
  taskbarAppOrder: string[];
  taskbarHiddenAppIds: string[];
  saveTaskbarAppOrder: (nextOrder: string[]) => void;
  saveTaskbarHiddenAppIds: (nextHidden: string[]) => void;
  saveTaskbarAppsLayout: (nextOrder: string[], nextHidden: string[]) => void;
  reorderTaskbarApps: (activeId: string, overId: string) => void;
}

function matchesBasePath(pathname: string, basePath: string): boolean {
  return pathname === basePath || pathname.startsWith(`${basePath}/`);
}

/**
 * Resolves which `ShellApp` owns a given pathname. Single source of truth
 * for "which app is active" — the shell chrome derives state from here via
 * `useActiveApp()` rather than mirroring it into store state.
 */
export function resolveActiveApp(pathname: string): ShellApp {
  return (
    registeredApps.find((a) => matchesBasePath(pathname, a.basePath)) ??
    registeredApps[0]
  );
}

function isPinnedTaskbarApp(app: ShellApp): boolean {
  return app.id === "desktop" || app.id === "profile";
}

function normalizeTaskbarAppOrder(apps: ShellApp[], savedIds: string[]): string[] {
  const defaultIds = apps.filter((app) => !isPinnedTaskbarApp(app)).map((app) => app.id);
  const knownIds = new Set(defaultIds);
  const normalizedIds: string[] = [];

  for (const id of savedIds) {
    if (!knownIds.has(id) || normalizedIds.includes(id)) continue;
    normalizedIds.push(id);
  }

  for (const id of defaultIds) {
    if (!normalizedIds.includes(id)) normalizedIds.push(id);
  }

  return normalizedIds;
}

function normalizeTaskbarHiddenAppIds(apps: ShellApp[], savedIds: string[]): string[] {
  const reorderableIds = new Set(
    apps.filter((app) => !isPinnedTaskbarApp(app)).map((app) => app.id),
  );
  const hidden: string[] = [];
  for (const id of savedIds) {
    if (!reorderableIds.has(id) || hidden.includes(id)) continue;
    hidden.push(id);
  }
  return hidden;
}

function getDefaultHiddenTaskbarAppIds(apps: ShellApp[]): string[] {
  return apps
    .filter((app) => app.defaultHidden && !isPinnedTaskbarApp(app))
    .map((app) => app.id);
}

function moveItem(ids: string[], fromIndex: number, toIndex: number): string[] {
  if (fromIndex === toIndex) return ids;
  const nextIds = [...ids];
  const [movedId] = nextIds.splice(fromIndex, 1);
  nextIds.splice(toIndex, 0, movedId);
  return nextIds;
}

function getInitialTaskbarAppOrder(): string[] {
  const savedIds = typeof window === "undefined" ? [] : getTaskbarAppOrder();
  return normalizeTaskbarAppOrder(registeredApps, savedIds);
}

function getInitialTaskbarHiddenAppIds(): string[] {
  const savedIds = typeof window === "undefined" ? null : getTaskbarHiddenAppIds();
  const seedIds = savedIds ?? getDefaultHiddenTaskbarAppIds(registeredApps);
  return normalizeTaskbarHiddenAppIds(registeredApps, seedIds);
}

export const useAppStore = create<AppState>()((set, get) => ({
  apps: registeredApps,
  taskbarAppOrder: getInitialTaskbarAppOrder(),
  taskbarHiddenAppIds: getInitialTaskbarHiddenAppIds(),
  saveTaskbarAppOrder: (nextOrder: string[]) => {
    const normalizedOrder = normalizeTaskbarAppOrder(get().apps, nextOrder);
    setTaskbarAppOrder(normalizedOrder);
    set({ taskbarAppOrder: normalizedOrder });
  },
  saveTaskbarHiddenAppIds: (nextHidden: string[]) => {
    const normalizedHidden = normalizeTaskbarHiddenAppIds(get().apps, nextHidden);
    setTaskbarHiddenAppIds(normalizedHidden);
    set({ taskbarHiddenAppIds: normalizedHidden });
  },
  saveTaskbarAppsLayout: (nextOrder: string[], nextHidden: string[]) => {
    const apps = get().apps;
    const normalizedOrder = normalizeTaskbarAppOrder(apps, nextOrder);
    const normalizedHidden = normalizeTaskbarHiddenAppIds(apps, nextHidden);
    setTaskbarAppOrder(normalizedOrder);
    setTaskbarHiddenAppIds(normalizedHidden);
    set({
      taskbarAppOrder: normalizedOrder,
      taskbarHiddenAppIds: normalizedHidden,
    });
  },
  reorderTaskbarApps: (activeId: string, overId: string) => {
    if (activeId === overId) return;

    const state = get();
    const currentOrder = normalizeTaskbarAppOrder(state.apps, state.taskbarAppOrder);
    const fromIndex = currentOrder.indexOf(activeId);
    const toIndex = currentOrder.indexOf(overId);

    if (fromIndex === -1 || toIndex === -1) return;

    const nextOrder = moveItem(currentOrder, fromIndex, toIndex);
    state.saveTaskbarAppOrder(nextOrder);
  },
}));

export function getOrderedTaskbarApps(apps: ShellApp[], taskbarAppOrder: string[]): ShellApp[] {
  const rank = new Map(taskbarAppOrder.map((id, index) => [id, index]));

  return [...apps].sort((a, b) => {
    const aRank = rank.get(a.id);
    const bRank = rank.get(b.id);

    if (aRank == null && bRank == null) return 0;
    if (aRank == null) return 1;
    if (bRank == null) return -1;
    return aRank - bRank;
  });
}
