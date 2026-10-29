import type { Condition } from "../../api";
import type { Slice } from "../store";

/** What the toolbar narrows a view to, and its unsaved filter changes (ADR 0038). None of it is saved: Save
 *  View writes the drafts to the view, and leaving the view drops all of it. */
export interface ViewNarrowing {
  /** "Filter this view". */
  text: string;
  /** Only children of this epic ("Open as list view"). */
  epic: string | null;
  /** The Me chip's draft; null shows the view's saved one. */
  me: boolean | null;
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
  /** Bumped to open the toolbar's Filter popover from elsewhere (an empty list's Edit filter). */
  filterRequests: number;
  requestFilter: () => void;
}

export const NO_VIEW_NARROWING: ViewNarrowing = { text: "", epic: null, me: null, list: null, view: null };

export const narrowingKey = (listId: number, viewId: number) => `${listId}:${viewId}`;

export const narrowingSlice: Slice<NarrowingSlice> = {
  create: (set) => ({
    narrowings: {},
    setNarrowing: (key, patch) =>
      set((s) => ({ narrowings: { ...s.narrowings, [key]: { ...(s.narrowings[key] ?? NO_VIEW_NARROWING), ...patch } } })),
    clearDrafts: (key) =>
      set((s) => {
        const current = s.narrowings[key];
        if (!current) return s;
        return { narrowings: { ...s.narrowings, [key]: { ...current, me: null, list: null, view: null } } };
      }),
    keepOnlyNarrowing: (key) =>
      set((s) => {
        const others = Object.keys(s.narrowings).filter((k) => k !== key);
        if (others.length === 0) return s;
        const kept = s.narrowings[key];
        return { narrowings: kept ? { [key]: kept } : {} };
      }),
    filterRequests: 0,
    requestFilter: () => set((s) => ({ filterRequests: s.filterRequests + 1 })),
  }),
};
