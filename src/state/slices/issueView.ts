import type { Slice } from "../store";

/** The view an Issue view was opened from; esc returns to it, its place intact. */
export interface IssueOrigin {
  listId: number;
  viewId: number;
}

export interface IssueViewState {
  origin: IssueOrigin;
  stack: string[];
}

/** The full Issue view, shown in place of the view: the issue on top, the ones it was opened from below. Each
 *  tab keeps its own: switching tabs parks the shown one and brings back the other's. */
export interface IssueViewSlice {
  issueView: IssueViewState | null;
  /** The Issue views of the tabs not shown, by tab id. */
  parkedIssueViews: Record<number, IssueViewState>;
  /** Parks the shown Issue view with tab `from` and shows tab `to`'s; tabs no longer open (`open`) lose theirs. */
  switchTab: (from: number, to: number, open: number[]) => void;
  /** Gives a tab that isn't shown an Issue view, shown when the tab is. */
  parkIssueView: (tab: number, view: IssueViewState) => void;
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
    parkedIssueViews: {},
    switchTab: (from, to, open) =>
      set((s) => {
        const parked: Record<number, IssueViewState> = {};
        for (const id of open) if (id !== to && s.parkedIssueViews[id]) parked[id] = s.parkedIssueViews[id];
        if (s.issueView && open.includes(from)) parked[from] = s.issueView;
        return { issueView: s.parkedIssueViews[to] ?? null, parkedIssueViews: parked };
      }),
    parkIssueView: (tab, view) => set((s) => ({ parkedIssueViews: { ...s.parkedIssueViews, [tab]: view } })),
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
