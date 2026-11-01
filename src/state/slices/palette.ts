import type { Slice } from "../store";

export interface PaletteSlice {
  /** The command palette (⌘K) is open. */
  paletteOpen: boolean;
  /** What the palette starts with (a `search` link); empty when opened by hand. */
  paletteText: string;
  setPaletteOpen: (open: boolean, text?: string) => void;
}

export const paletteSlice: Slice<PaletteSlice> = {
  create: (set) => ({
    paletteOpen: false,
    paletteText: "",
    setPaletteOpen: (paletteOpen, paletteText = "") => set({ paletteOpen, paletteText }),
  }),
};
