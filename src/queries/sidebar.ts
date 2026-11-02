import { useCallback, useEffect } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type Nav } from "../api";
import { fail } from "../state/requests";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";

export const SIDEBAR_KEY = ["sidebar"] as const;
export const NAV_KEY = ["nav"] as const;
/** Every level's views; `viewsKey(id)` is one level's. */
export const VIEWS_KEY = ["views"] as const;
export const viewsKey = (listId: number) => [...VIEWS_KEY, listId] as const;

/** Everything, Home and the Spaces with their filter lists and open counts. */
export const useSidebar = () => useQuery({ queryKey: SIDEBAR_KEY, queryFn: api.sidebarTree });
/** The tracked Spaces in sidebar order. */
export const useSpaces = () => useQuery({ queryKey: SIDEBAR_KEY, queryFn: api.sidebarTree, select: (tree) => tree.spaces });

/** Where the main window is and which Spaces are expanded. */
export const useNav = () => useQuery({ queryKey: NAV_KEY, queryFn: api.navCurrent });

/** The view tabs of a level; nothing while there is no level. */
export const useViews = (listId: number | null) =>
  useQuery({ queryKey: viewsKey(listId ?? -1), queryFn: () => api.viewsOf(listId ?? -1), enabled: listId !== null });

/** Opens a level (at `viewId`, or its last or default view) and takes Rust's new place into the cache. Going
 *  to the place already shown brings back its view: the Issue view and the peek close. */
export function useNavGo() {
  const client = useQueryClient();
  return useCallback(
    (listId: number, viewId: number | null = null) => {
      const before = client.getQueryData<Nav>(NAV_KEY)?.location;
      return api.navGo(listId, viewId).then(
        (nav) => {
          client.setQueryData<Nav>(NAV_KEY, nav);
          const at = nav.location;
          if (!at || !before || at.listId !== before.listId || at.viewId !== before.viewId) return;
          const store = useStore.getState();
          store.closeIssue();
          if (store.places[narrowingKey(at.listId, at.viewId)]?.peekOpen) store.updatePlace(at.listId, at.viewId, { peekOpen: false });
        },
        (e: unknown) => void fail(e, "Could not open it"),
      );
    },
    [client],
  );
}

/** Goes back or forward in the active tab: to that place, with its issue open in the Issue view or none. */
export function useTravel() {
  const client = useQueryClient();
  return useCallback(
    (direction: "back" | "forward") =>
      (direction === "back" ? api.navBack() : api.navForward()).then(
        (nav) => {
          client.setQueryData<Nav>(NAV_KEY, nav);
          const store = useStore.getState();
          if (nav.issue && nav.location) store.openIssue(nav.issue, { listId: nav.location.listId, viewId: nav.location.viewId });
          else store.closeIssue();
        },
        (e: unknown) => void fail(e, "Could not go there"),
      ),
    [client],
  );
}

/** Tells Rust which issue the Issue view shows, so it is a step of the tab's history. */
export function useReportShownIssue(issue: string | null) {
  const client = useQueryClient();
  useEffect(() => {
    // The reply may be older than a move made meanwhile, so the place is asked again instead.
    void api.navSetIssue(issue).then(
      () => client.invalidateQueries({ queryKey: NAV_KEY }),
      () => {},
    );
  }, [client, issue]);
}

/** Saves which Spaces are expanded. */
export function useSetExpanded() {
  const client = useQueryClient();
  return useCallback(
    (expanded: number[]) => {
      // Shown at once; Rust's answer replaces it.
      client.setQueryData<Nav>(NAV_KEY, (nav) => (nav ? { ...nav, expanded } : nav));
      return api.navSetExpanded(expanded).then(
        (nav) => void client.setQueryData<Nav>(NAV_KEY, nav),
        (e: unknown) => void fail(e, "Could not save the sidebar"),
      );
    },
    [client],
  );
}

/** The tab actions; each takes Rust's new place and tabs into the cache. */
export function useTabs() {
  const client = useQueryClient();
  const take = useCallback(
    (call: Promise<Nav>, failure: string) =>
      call.then(
        (nav) => void client.setQueryData<Nav>(NAV_KEY, nav),
        (e: unknown) => void fail(e, failure),
      ),
    [client],
  );
  return {
    /** A new tab at the level (and view, else its last one); `activate` shows it, else it waits in the strip. */
    open: useCallback(
      (listId: number, viewId: number | null, activate: boolean) => take(api.navTabNew(listId, viewId, activate), "Could not open a tab"),
      [take],
    ),
    activate: useCallback((id: number) => take(api.navTabActivate(id), "Could not show the tab"), [take]),
    close: useCallback((id: number) => take(api.navTabClose(id), "Could not close the tab"), [take]),
    move: useCallback((id: number, before: number | null) => take(api.navTabMove(id, before), "Could not move the tab"), [take]),
  };
}
