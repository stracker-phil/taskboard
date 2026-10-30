import type { Slice } from "../store";

/** The view an Issue view was opened from; esc returns to it, its place intact. */
export interface IssueOrigin {
  listId: number;
  viewId: number;
}

/** The full Issue view, shown in place of the view: the issue on top, the ones it was opened from below. */
export interface IssueViewSlice {
  issueView: { origin: IssueOrigin; stack: string[] } | null;
  openIssue: (id: string, origin: IssueOrigin) => void;
  /** Opens a related issue (an epic, a child, a link) on top; back returns to the one before. */
  openRelated: (id: string) => void;
  /** Swaps the shown issue for its neighbor in the view (previous/next); the view's selection follows. */
  showIssue: (id: string) => void;
  /** One step back: the previous issue, or the view. */
  issueBack: () => void;
  closeIssue: () => void;
}

export const issueViewSlice: Slice<IssueViewSlice> = {
  create: (set, get) => ({
    issueView: null,
    openIssue: (id, origin) => set({ issueView: { origin, stack: [id] } }),
    openRelated: (id) =>
      set((s) => (s.issueView && s.issueView.stack.at(-1) !== id ? { issueView: { ...s.issueView, stack: [...s.issueView.stack, id] } } : s)),
    showIssue: (id) => {
      const view = get().issueView;
      if (!view) return;
      set({ issueView: { ...view, stack: [...view.stack.slice(0, -1), id] } });
      get().updatePlace(view.origin.listId, view.origin.viewId, { selectedId: id });
    },
    issueBack: () =>
      set((s) => {
        if (!s.issueView || s.issueView.stack.length <= 1) return { issueView: null };
        return { issueView: { ...s.issueView, stack: s.issueView.stack.slice(0, -1) } };
      }),
    closeIssue: () => set({ issueView: null }),
  }),
};
