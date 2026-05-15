import type { GridStatus } from "../types";
import { apiFetch } from "./core";

export const gridApi = {
  status: () => apiFetch<GridStatus>("/api/grid/status"),

  setConfig: (params: { multiaddr: string }) =>
    apiFetch<GridStatus>("/api/grid/config", {
      method: "POST",
      body: JSON.stringify(params),
    }),

  /**
   * Persist the connect-timeout (in milliseconds) used on the next
   * GRID dial. Pass `null` to clear any custom value and revert to the
   * SDK default (currently 30 000 ms). Server enforces 1 000–300 000 ms.
   */
  setTimeout: (params: { timeout_ms: number | null }) =>
    apiFetch<GridStatus>("/api/grid/timeout", {
      method: "POST",
      body: JSON.stringify(params),
    }),

  connect: () =>
    apiFetch<GridStatus>("/api/grid/connect", { method: "POST" }),

  disconnect: () =>
    apiFetch<GridStatus>("/api/grid/disconnect", { method: "POST" }),
};
