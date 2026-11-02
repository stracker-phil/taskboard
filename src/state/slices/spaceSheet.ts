import type { Slice } from "../store";

export interface SpaceSheetSlice {
  /** The Space (its project id) whose settings show in the main window's sheet. */
  spaceSheet: string | null;
  setSpaceSheet: (projectId: string | null) => void;
}

export const spaceSheetSlice: Slice<SpaceSheetSlice> = {
  create: (set) => ({
    spaceSheet: null,
    setSpaceSheet: (spaceSheet) => set({ spaceSheet }),
  }),
};
