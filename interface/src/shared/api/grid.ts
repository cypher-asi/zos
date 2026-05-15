import type { GridStatus } from "../types";
import { apiFetch } from "./core";

export const gridApi = {
  status: () => apiFetch<GridStatus>("/api/grid/status"),

  setConfig: (params: { multiaddr: string }) =>
    apiFetch<GridStatus>("/api/grid/config", {
      method: "POST",
      body: JSON.stringify(params),
    }),

  connect: () =>
    apiFetch<GridStatus>("/api/grid/connect", { method: "POST" }),

  disconnect: () =>
    apiFetch<GridStatus>("/api/grid/disconnect", { method: "POST" }),
};
