import type { Narrowing } from "../../api";
import type { Slice } from "../store";

/** The toolbar's Me chip, text filter and epic narrowing, per view. The Me chip and the text are kept with the
 *  view's place (`places.json`); the epic narrowing isn't. */
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
  create: (set, get) => ({
    narrowings: {},
    setNarrowing: (key, patch) => {
      set((s) => ({ narrowings: { ...s.narrowings, [key]: { ...(s.narrowings[key] ?? NO_NARROWING), ...patch } } }));
      // The Me chip and the text stay with the view (its saved place); an epic narrowing is for the moment.
      if (patch.me === undefined && patch.text === undefined) return;
      const [listId, viewId] = key.split(":").map(Number);
      const { me, text } = get().narrowings[key];
      get().updatePlace(listId, viewId, { onlyMine: me, text });
    },
    filterRequests: 0,
    requestFilter: () => set((s) => ({ filterRequests: s.filterRequests + 1 })),
  }),
};
