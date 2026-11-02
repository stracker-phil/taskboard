import { useQueryClient } from "@tanstack/react-query";
import { useCallback, type MouseEvent } from "react";
import { api, type Nav } from "../api";
import { NAV_KEY, useTabs } from "../queries/sidebar";
import { attempt } from "../state/requests";
import type { IssueOrigin } from "../state/slices/issueView";
import { useStore } from "../state/store";

/** ⌘-click on a sidebar row, a view tab or a bookmark opens its place in a new tab behind the shown one; ⌘⇧-click
 *  opens it and shows it (ADR 0050). Answers whether the click opened a tab, so the control skips its own action. */
export function useOpenInTab() {
  const { open } = useTabs();
  return useCallback(
    (e: MouseEvent, listId: number, viewId: number | null = null) => {
      if (!e.metaKey) return false;
      void open(listId, viewId, e.shiftKey);
      return true;
    },
    [open],
  );
}

/** Opens the issue's Issue view in a new tab at the view it was opened from, behind the shown tab or, with
 *  `show`, shown. */
export function useOpenIssueInTab() {
  const client = useQueryClient();
  const { activate } = useTabs();
  return useCallback(
    async (id: string, origin: IssueOrigin, show: boolean) => {
      const nav = await attempt(api.navTabNew(origin.listId, origin.viewId, false), "Could not open a tab");
      if (nav === null) return;
      // A new tab comes right after the shown one.
      const tab = nav.tabs[nav.tabs.findIndex((t) => t.id === nav.activeTab) + 1];
      if (!tab) return;
      useStore.getState().parkIssueView(tab.id, { origin, stack: [id] });
      client.setQueryData<Nav>(NAV_KEY, nav);
      if (show) await activate(tab.id);
    },
    [client, activate],
  );
}
