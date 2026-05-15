import type { IdentityDto } from "../types";
import { apiFetch } from "./core";

export const identityApi = {
  get: () => apiFetch<IdentityDto | null>("/api/identity"),

  create: () =>
    apiFetch<IdentityDto>("/api/identity", { method: "POST" }),
};
