import { create } from "zustand";
import type { Toast } from "@cypher-asi/zui";

interface UIState {
  sidekickTab: string;
  setSidekickTab: (tab: string) => void;

  modalOpen: string | null;
  openModal: (id: string) => void;
  closeModal: () => void;

  toasts: Toast[];
  addToast: (toast: Omit<Toast, "id">) => void;
  removeToast: (id: string) => void;
}

let toastId = 0;

export const useUIStore = create<UIState>()((set) => ({
  sidekickTab: "details",
  setSidekickTab: (tab) => set({ sidekickTab: tab }),

  modalOpen: null,
  openModal: (id) => set({ modalOpen: id }),
  closeModal: () => set({ modalOpen: null }),

  toasts: [],
  addToast: (toast) =>
    set((s) => ({ toasts: [...s.toasts, { ...toast, id: String(++toastId) }] })),
  removeToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));
