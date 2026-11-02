import type { Condition } from "../../api";
import type { Slice } from "../store";

/** What the toolbar narrows a view to, and its unsaved changes (ADR 0038). None of it is saved: Save View writes
 *  the drafts (Me, person, conditions) to the view, and leaving the view drops all of it. */
export interface ViewNarrowing {
  /** "Filter this view". */
  text: string;
  /** Only children of this epic ("Open as list view"). */
  epic: string | null;
  /** The Me chip's draft; null shows the view's saved one. */
  me: boolean | null;
  /** The person chip's draft: an account, null for anyone; undefined shows the view's saved one. */
  person?: string | null;
  /** Draft conditions in place of the filter list's own ones, as edited (incomplete ones too); null keeps the
   *  saved ones. */
  list: Condition[] | null;
  /** Draft conditions in place of the view's own ones. */
  view: Condition[] | null;
}

export interface NarrowingSlice {
  /** By `narrowingKey`. */
  narrowings: Record<string, ViewNarrowing>;
  setNarrowing: (key: string, patch: Partial<ViewNarrowing>) => void;
  /** Back to what the view saved: after Save View, or Revert View. The text and the epic stay. */
  clearDrafts: (key: string) => void;
  /** Drops every other view's narrowing and drafts: the window left them. */
  keepOnlyNarrowing: (key: string) => void;
  /** Keeps the narrowings and drafts of the views the open tabs show; the rest were left. */
  keepNarrowings: (keys: string[]) => void;
  /** Bumped to open the toolbar's Filter popover from elsewhere (an empty list's Edit filter). */
  filterRequests: number;
  requestFilter: () => void;
}

export const NO_VIEW_NARROWING: ViewNarrowing = { text: "", epic: null, me: null, person: undefined, list: null, view: null };

export const narrowingKey = (listId: number, viewId: number) => `${listId}:${viewId}`;

export const narrowingSlice: Slice<NarrowingSlice> = {
  create: (set, get) => ({
    narrowings: {},
    setNarrowing: (key, patch) =>
      set((s) => ({ narrowings: { ...s.narrowings, [key]: { ...(s.narrowings[key] ?? NO_VIEW_NARROWING), ...patch } } })),
    clearDrafts: (key) =>
      set((s) => {
        const current = s.narrowings[key];
        if (!current) return s;
        return { narrowings: { ...s.narrowings, [key]: { ...current, me: null, person: undefined, list: null, view: null } } };
      }),
    keepOnlyNarrowing: (key) => get().keepNarrowings([key]),
    keepNarrowings: (keys) =>
      set((s) => {
        if (Object.keys(s.narrowings).every((k) => keys.includes(k))) return s;
        return { narrowings: Object.fromEntries(Object.entries(s.narrowings).filter(([k]) => keys.includes(k))) };
      }),
    filterRequests: 0,
    requestFilter: () => set((s) => ({ filterRequests: s.filterRequests + 1 })),
  }),
};
