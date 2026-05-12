import { useState, useEffect } from "react";

declare global {
  interface Window {
    ipc?: { postMessage(msg: string): void };
  }
}

export function windowCommand(cmd: string) {
  window.ipc?.postMessage(cmd);
}

interface DesktopBridge {
  hasDesktopBridge: boolean;
  windowControls: boolean;
}

function read(): DesktopBridge {
  const has = typeof window !== "undefined" && typeof window.ipc?.postMessage === "function";
  return { hasDesktopBridge: has, windowControls: has };
}

export function useDesktopBridge(): DesktopBridge {
  const [bridge, setBridge] = useState(read);
  useEffect(() => setBridge(read()), []);
  return bridge;
}
