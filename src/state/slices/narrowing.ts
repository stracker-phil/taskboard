import type { Narrowing } from "../../api";
import type { Slice } from "../store";

/** The toolbar's Me chip and text filter, per view and never saved: a narrowing worth keeping becomes a filter
 *  list. */
export interface NarrowingSlice {
  /** By `narrowingKey`. */
  narrowings: Record<string, Narrowing>;
  setNarrowing: (key: string, patch: Partial<Narrowing>) => void;
}

export const NO_NARROWING: Narrowing = { me: false, text: "", epic: null };

export const narrowingKey = (listId: number, viewId: number) => `${listId}:${viewId}`;

export const narrowingSlice: Slice<NarrowingSlice> = {
  create: (set) => ({
    narrowings: {},
    setNarrowing: (key, patch) =>
      set((s) => ({ narrowings: { ...s.narrowings, [key]: { ...(s.narrowings[key] ?? NO_NARROWING), ...patch } } })),
  }),
};
