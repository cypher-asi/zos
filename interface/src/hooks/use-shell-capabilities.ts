import { useSyncExternalStore } from "react";

/**
 * Subset of aura-os's `useAuraCapabilities` that this shell actually needs:
 * we just want to know whether the desktop bridge (i.e. the Wry IPC channel
 * defined by `apps/zero-desktop`) is available so chrome can show window
 * controls. No mobile/standalone/native breakpoint logic.
 */
export interface ShellCapabilities {
  hasDesktopBridge: boolean;
  features: {
    windowControls: boolean;
  };
}

declare global {
  interface Window {
    ipc?: { postMessage(msg: string): void };
  }
}

function read(): ShellCapabilities {
  const has =
    typeof window !== "undefined" &&
    typeof window.ipc?.postMessage === "function";
  return {
    hasDesktopBridge: has,
    features: { windowControls: has },
  };
}

let snapshot: ShellCapabilities = read();
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  if (listeners.size === 1 && typeof window !== "undefined") {
    window.addEventListener("focus", recompute);
  }
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && typeof window !== "undefined") {
      window.removeEventListener("focus", recompute);
    }
  };
}

function recompute() {
  const next = read();
  if (
    snapshot.hasDesktopBridge === next.hasDesktopBridge &&
    snapshot.features.windowControls === next.features.windowControls
  ) {
    return;
  }
  snapshot = next;
  for (const listener of listeners) listener();
}

function getSnapshot(): ShellCapabilities {
  return snapshot;
}

export function useShellCapabilities(): ShellCapabilities {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
