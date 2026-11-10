import { create } from "zustand";

interface InboxState {
  open: boolean;
  /** The selected row's index. */
  selected: number;
  /** The selected row shows its peek card beside the inbox. */
  previewing: boolean;
  setOpen: (open: boolean) => void;
  select: (index: number) => void;
  setPreviewing: (previewing: boolean) => void;
}

/** Whether the inbox is open, which row is selected and whether it is previewed; the rows themselves come from
 *  Rust (`inbox_rows`). */
export const useInboxState = create<InboxState>()((set) => ({
  open: false,
  selected: 0,
  previewing: false,
  setOpen: (open) => set({ open, selected: 0, previewing: false }),
  select: (selected) => set({ selected }),
  setPreviewing: (previewing) => set({ previewing }),
}));
