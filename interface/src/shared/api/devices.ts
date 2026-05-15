import type { DeviceDto } from "../types";
import { apiFetch } from "./core";

export const devicesApi = {
  list: () => apiFetch<DeviceDto[]>("/api/devices"),

  create: (params: { label?: string | null; capabilities: number }) =>
    apiFetch<DeviceDto>("/api/devices", {
      method: "POST",
      body: JSON.stringify({
        label: params.label ?? null,
        capabilities: params.capabilities,
      }),
    }),
};
