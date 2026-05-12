import { apiFetch } from "./core";

export interface Project {
  id: string;
  name: string;
  description: string;
  updatedAt: string;
}

export interface CreateProjectInput {
  name: string;
  description?: string;
}

export const projectsApi = {
  list: () => apiFetch<Project[]>("/api/projects"),
  create: (data: CreateProjectInput) =>
    apiFetch<Project>("/api/projects", {
      method: "POST",
      body: JSON.stringify(data),
    }),
  get: (id: string) => apiFetch<Project>(`/api/projects/${id}`),
  update: (id: string, data: Partial<Pick<Project, "name" | "description">>) =>
    apiFetch<Project>(`/api/projects/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    }),
  delete: (id: string) =>
    apiFetch<void>(`/api/projects/${id}`, { method: "DELETE" }),
};
