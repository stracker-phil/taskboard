import { api } from "../../api";
import { fail } from "../requests";
import type { Slice } from "../store";

export interface Toast {
  id: number;
  kind: "error" | "info" | "success";
  message: string;
  /** How many times the same message was reported while it was showing. */
  count: number;
  /** A button in the toast, e.g. Undo; running it dismisses the toast. */
  action?: ToastAction;
}

export interface ToastAction {
  label: string;
  run: () => void;
  /** Overrides how long the toast stays. */
  ttlMs?: number;
}

export interface UiSlice {
  /** The main window's sidebar is hidden (`ui_state.json` `sidebarHidden`). */
  sidebarHidden: boolean;
  loadUiState: () => Promise<void>;
  toggleSidebar: () => void;
  toasts: Toast[];
  toast: (kind: Toast["kind"], message: string, action?: ToastAction) => void;
  dismissToast: (id: number) => void;
}

let nextToastId = 1;
const MAX_TOASTS = 5;

export const uiSlice: Slice<UiSlice> = {
  create: (set, get) => ({
    sidebarHidden: false,
    toasts: [],

    async loadUiState() {
      try {
        const ui = await api.getUiState();
        set({ sidebarHidden: ui.sidebarHidden });
      } catch (e) {
        fail(e, "Could not load the window layout");
      }
    },

    toggleSidebar() {
      const sidebarHidden = !get().sidebarHidden;
      set({ sidebarHidden });
      void api.setUiState({ sidebarHidden }).catch((e) => fail(e, "Could not save the window layout"));
    },

    toast(kind, message, action) {
      if (kind === "error") void api.logError(message).catch(() => {});
      const same = get().toasts.find((t) => t.kind === kind && t.message === message);
      if (same) {
        set((s) => ({ toasts: s.toasts.map((t) => (t.id === same.id ? { ...t, count: t.count + 1 } : t)) }));
        return;
      }
      const id = nextToastId++;
      set((s) => ({ toasts: [...s.toasts, { id, kind, message, count: 1, action }].slice(-MAX_TOASTS) }));
    },

    dismissToast(id) {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    },
  }),
};
