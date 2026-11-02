import { useCallback, type MouseEvent } from "react";
import { useTabs } from "../queries/sidebar";

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
