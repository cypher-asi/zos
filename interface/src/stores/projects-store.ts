import { create } from "zustand";

interface ProjectsUIState {
  selectedId: string | null;
  selectProject: (id: string | null) => void;
}

export const useProjectsStore = create<ProjectsUIState>()((set) => ({
  selectedId: null,
  selectProject: (id) => set({ selectedId: id }),
}));
