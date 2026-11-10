import { create } from "zustand";

interface InboxState {
  open: boolean;
  /** The selected row's index. */
  selected: number;
  setOpen: (open: boolean) => void;
  select: (index: number) => void;
}

/** Whether the inbox is open and which row is selected; the rows themselves come from Rust (`inbox_rows`). */
export const useInboxState = create<InboxState>()((set) => ({
  open: false,
  selected: 0,
  setOpen: (open) => set({ open, selected: 0 }),
  select: (selected) => set({ selected }),
}));
