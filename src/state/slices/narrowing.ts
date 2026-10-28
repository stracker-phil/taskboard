import type { Narrowing } from "../../api";
import type { Slice } from "../store";

/** The toolbar's Me chip and text filter, per view and never saved: a narrowing worth keeping becomes a filter
 *  list. */
export interface NarrowingSlice {
  /** By `narrowingKey`. */
  narrowings: Record<string, Narrowing>;
  setNarrowing: (key: string, patch: Partial<Narrowing>) => void;
  /** Bumped to open the toolbar's Filter popover from elsewhere (an empty list's Edit filter). */
  filterRequests: number;
  requestFilter: () => void;
}

export const NO_NARROWING: Narrowing = { me: false, text: "", epic: null };

export const narrowingKey = (listId: number, viewId: number) => `${listId}:${viewId}`;

export const narrowingSlice: Slice<NarrowingSlice> = {
  create: (set) => ({
    narrowings: {},
    setNarrowing: (key, patch) =>
      set((s) => ({ narrowings: { ...s.narrowings, [key]: { ...(s.narrowings[key] ?? NO_NARROWING), ...patch } } })),
    filterRequests: 0,
    requestFilter: () => set((s) => ({ filterRequests: s.filterRequests + 1 })),
  }),
};
