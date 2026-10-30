import { type RefObject, useCallback, useEffect, useRef } from "react";
import type { IssueOpening } from "../../api";
import { useDoubleClickMs } from "../../queries/useDoubleClickMs";
import type { RowKeyState } from "./useRowKeys";

/** A row's click and double-click for the List and Table. A click selects at once; when it opens the preview,
 *  the preview waits out the double-click interval, so a double-click opens the Issue view alone. */
export function useRowClicks(keys: RefObject<RowKeyState>, opening: IssueOpening) {
  const interval = useDoubleClickMs();
  const pendingPreview = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => {
    const pending = pendingPreview;
    return () => clearTimeout(pending.current);
  }, []);

  const onSelect = useCallback(
    (id: string) => {
      clearTimeout(pendingPreview.current);
      const { select, open, peekOpen } = keys.current;
      if (opening === "full") {
        select(id, false);
        open(id);
        return;
      }
      // An open preview just follows the selection.
      select(id, peekOpen);
      if (peekOpen) return;
      pendingPreview.current = setTimeout(() => {
        if (keys.current.selected === id) keys.current.select(id, true);
      }, interval);
    },
    [keys, opening, interval],
  );
  const onOpen = useCallback(
    (id: string) => {
      clearTimeout(pendingPreview.current);
      keys.current.open(id);
    },
    [keys],
  );
  return { onSelect, onOpen };
}
