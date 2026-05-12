import { api } from "./client";

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
  list: () => api.get<Project[]>("/projects"),
  create: (data: CreateProjectInput) => api.post<Project>("/projects", data),
  get: (id: string) => api.get<Project>(`/projects/${id}`),
  update: (id: string, data: Partial<Pick<Project, "name" | "description">>) =>
    api.put<Project>(`/projects/${id}`, data),
  delete: (id: string) => api.del<void>(`/projects/${id}`),
};
