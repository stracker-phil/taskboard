import { type RefObject, useCallback, useEffect, useRef } from "react";
import type { IssueOpening } from "../../api";
import { useDoubleClickMs } from "../../queries/useDoubleClickMs";
import { pickRange, toggleBox, togglePick } from "./picking";
import type { RowKeyState } from "./useRowKeys";

/** The modifier keys of a row click that pick rows: ⌘ adds or removes one, ⇧ picks a range. */
export interface PickKeys {
  metaKey: boolean;
  shiftKey: boolean;
  /** The click's count: 2 for the second click of a double-click. */
  detail?: number;
}

interface PickState {
  picked: string[];
  selected: string | null;
  peekOpen: boolean;
}

/** A row's click and double-click for the List and Table. A click selects at once; when it opens the preview,
 *  the preview waits out the double-click interval, so a double-click opens the Issue view alone. ⌘ and ⇧ clicks
 *  and the checkbox pick rows instead and open nothing; a plain click drops the picks. A ⌘ double-click opens the
 *  issue in a new tab (⌘⇧ shows it) and leaves the picks as they were before it. */
export function useRowClicks(keys: RefObject<RowKeyState>, opening: IssueOpening) {
  const interval = useDoubleClickMs();
  const pendingPreview = useRef<ReturnType<typeof setTimeout>>(undefined);
  const beforeMetaClick = useRef<PickState | null>(null);
  useEffect(() => {
    const pending = pendingPreview;
    return () => clearTimeout(pending.current);
  }, []);

  const onSelect = useCallback(
    (id: string, mods?: PickKeys) => {
      clearTimeout(pendingPreview.current);
      const { select, open, peekOpen, rowIds, picked, selected, setPicked } = keys.current;
      if (mods?.metaKey) {
        if ((mods.detail ?? 1) <= 1) beforeMetaClick.current = { picked, selected, peekOpen };
        setPicked(togglePick(rowIds, picked, id, selected));
        select(id, false);
        return;
      }
      if (mods?.shiftKey) {
        setPicked(pickRange(rowIds, selected, id));
        if (selected === null) select(id, false);
        return;
      }
      if (picked.length > 0) setPicked([]);
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
    (id: string, mods?: PickKeys) => {
      clearTimeout(pendingPreview.current);
      if (!mods?.metaKey) {
        keys.current.open(id);
        return;
      }
      const before = beforeMetaClick.current;
      if (before) {
        keys.current.setPicked(before.picked);
        keys.current.setPlace({ selectedId: before.selected, peekOpen: before.peekOpen });
      }
      keys.current.openInTab(id, mods.shiftKey);
    },
    [keys],
  );
  /** Selects the row and nothing else: a cell that opens a menu acts on it without opening it. */
  const onPick = useCallback(
    (id: string) => {
      clearTimeout(pendingPreview.current);
      const { picked, setPicked, select, peekOpen } = keys.current;
      if (picked.length > 0) setPicked([]);
      select(id, peekOpen);
    },
    [keys],
  );
  /** A row's checkbox: picks or unpicks that row alone. */
  const onToggleBox = useCallback(
    (id: string) => {
      clearTimeout(pendingPreview.current);
      const { rowIds, picked, setPicked } = keys.current;
      setPicked(toggleBox(rowIds, picked, id));
    },
    [keys],
  );
  return { onSelect, onOpen, onPick, onToggleBox };
}
