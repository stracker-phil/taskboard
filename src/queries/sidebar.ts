import { useCallback } from "react";
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
