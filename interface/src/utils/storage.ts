/**
 * Subset of aura-os's `utils/storage.ts` that the floating shell chrome
 * needs: last-app pointer, taskbar app order/hidden ids, and the
 * collapse-state flag for the taskbar's middle app rail.
 */

const LAST_APP_KEY = "shell:lastApp";
const TASKBAR_APP_ORDER_KEY = "shell:taskbarAppOrder";
const TASKBAR_HIDDEN_APPS_KEY = "shell:taskbarHiddenApps";
const TASKBAR_APPS_COLLAPSED_KEY = "shell:taskbarAppsCollapsed";
const PREVIOUS_PATH_KEY = "shell:previousPath";

export const STORAGE_KEYS = {
  LAST_APP: LAST_APP_KEY,
  TASKBAR_APP_ORDER: TASKBAR_APP_ORDER_KEY,
  TASKBAR_HIDDEN_APPS: TASKBAR_HIDDEN_APPS_KEY,
  TASKBAR_APPS_COLLAPSED: TASKBAR_APPS_COLLAPSED_KEY,
  PREVIOUS_PATH: PREVIOUS_PATH_KEY,
} as const;

export function getLastApp(): string | null {
  try {
    return localStorage.getItem(LAST_APP_KEY);
  } catch {
    return null;
  }
}

export function setLastApp(appId: string): void {
  try {
    localStorage.setItem(LAST_APP_KEY, appId);
  } catch {
    // ignore storage failures
  }
}

export function getPreviousPath(): string | null {
  try {
    return localStorage.getItem(PREVIOUS_PATH_KEY);
  } catch {
    return null;
  }
}

export function setPreviousPath(path: string): void {
  try {
    localStorage.setItem(PREVIOUS_PATH_KEY, path);
  } catch {
    // ignore storage failures
  }
}

export function getTaskbarAppsCollapsed(): boolean {
  try {
    const raw = localStorage.getItem(TASKBAR_APPS_COLLAPSED_KEY);
    if (raw === "true") return true;
    if (raw === "false") return false;
  } catch {
    // ignore storage failures
  }
  return false;
}

export function setTaskbarAppsCollapsed(collapsed: boolean): void {
  try {
    localStorage.setItem(TASKBAR_APPS_COLLAPSED_KEY, String(collapsed));
  } catch {
    // ignore storage failures
  }
}

export function getTaskbarAppOrder(): string[] {
  try {
    const raw = localStorage.getItem(TASKBAR_APP_ORDER_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      return parsed.filter((value): value is string => typeof value === "string");
    }
  } catch {
    // ignore malformed data
  }
  return [];
}

export function setTaskbarAppOrder(ids: string[]): void {
  try {
    if (ids.length === 0) {
      localStorage.removeItem(TASKBAR_APP_ORDER_KEY);
      return;
    }
    localStorage.setItem(TASKBAR_APP_ORDER_KEY, JSON.stringify(ids));
  } catch {
    // ignore storage failures
  }
}

/**
 * `null` => the user has never customized the hidden list (callers should
 * seed from the registry's `defaultHidden` flags). An empty array means the
 * user actively cleared the list, and we honor that.
 */
export function getTaskbarHiddenAppIds(): string[] | null {
  try {
    const raw = localStorage.getItem(TASKBAR_HIDDEN_APPS_KEY);
    if (raw === null) return null;
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) {
      return parsed.filter((value): value is string => typeof value === "string");
    }
  } catch {
    // ignore malformed data
  }
  return [];
}

export function setTaskbarHiddenAppIds(ids: string[]): void {
  try {
    localStorage.setItem(TASKBAR_HIDDEN_APPS_KEY, JSON.stringify(ids));
  } catch {
    // ignore storage failures
  }
}
