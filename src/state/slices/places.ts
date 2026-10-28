import { api, type ViewPlace } from "../../api";
import { fail } from "../requests";
import type { Slice } from "../store";
import { narrowingKey } from "./narrowing";

/** Where the owner left each view: open groups, scroll, the selected row and its peek. Rust keeps it across
 *  restarts (`places.json`); changes save a moment after the last one. */
export interface PlacesSlice {
  /** By `narrowingKey`; a view is here once its saved place has loaded. */
  places: Record<string, ViewPlace>;
  loadPlace: (listId: number, viewId: number) => Promise<void>;
  updatePlace: (listId: number, viewId: number, patch: Partial<ViewPlace>) => void;
}

export const EMPTY_PLACE: ViewPlace = { scrollTop: 0, toggled: {}, selectedId: null, peekOpen: false };

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
      // A change made while loading wins over the saved place.
      set((s) => (key in s.places ? s : { places: { ...s.places, [key]: saved ?? EMPTY_PLACE } }));
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
