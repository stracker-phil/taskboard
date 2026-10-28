import type { Slice } from "../store";

export interface PaletteSlice {
  /** The command palette (⌘K) is open. */
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
}

export const paletteSlice: Slice<PaletteSlice> = {
  create: (set) => ({
    paletteOpen: false,
    setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  }),
};
