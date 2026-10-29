import { api, type ViewPlace } from "../../api";
import { fail } from "../requests";
import type { Slice } from "../store";
import { narrowingKey, NO_NARROWING } from "./narrowing";

/** Where the owner left each view: open groups, scroll, the selected row and its peek. Rust keeps it across
 *  restarts (`places.json`); changes save a moment after the last one. */
export interface PlacesSlice {
  /** By `narrowingKey`; a view is here once its saved place has loaded. */
  places: Record<string, ViewPlace>;
  loadPlace: (listId: number, viewId: number) => Promise<void>;
  updatePlace: (listId: number, viewId: number, patch: Partial<ViewPlace>) => void;
}

export const EMPTY_PLACE: ViewPlace = { scrollTop: 0, toggled: {}, collapsedParents: [], selectedId: null, peekOpen: false, onlyMine: false, text: "" };

const SAVE_DELAY_MS = 300;
const pending = new Map<string, ReturnType<typeof setTimeout>>();

export const placesSlice: Slice<PlacesSlice> = {
  create: (set, get) => ({
    places: {},
    async loadPlace(listId, viewId) {
      const key = narrowingKey(listId, viewId);
      if (key in get().places) return;
      let saved: ViewPlace | null = null;
      try {
        saved = await api.placeGet(listId, viewId);
      } catch (e) {
        fail(e, "Could not restore where you were in this view");
      }
      // A change made while loading wins over the saved place; the view's Me chip and text come back with it.
      set((s) => {
        if (key in s.places) return s;
        const place = saved ?? EMPTY_PLACE;
        const narrowing = s.narrowings[key] ?? { ...NO_NARROWING, me: place.onlyMine, text: place.text };
        return { places: { ...s.places, [key]: place }, narrowings: { ...s.narrowings, [key]: narrowing } };
      });
    },
    updatePlace(listId, viewId, patch) {
      const key = narrowingKey(listId, viewId);
      set((s) => ({ places: { ...s.places, [key]: { ...(s.places[key] ?? EMPTY_PLACE), ...patch } } }));
      clearTimeout(pending.get(key));
      pending.set(
        key,
        setTimeout(() => {
          pending.delete(key);
          const place = get().places[key];
          if (place) void api.placeSet(listId, viewId, place).catch((e: unknown) => fail(e, "Could not remember where you were"));
        }, SAVE_DELAY_MS),
      );
    },
  }),
};
