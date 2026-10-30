import { type RefObject, useEffect, useRef } from "react";
import type { Group, ViewData, ViewPlace } from "../../api";
import { onIssueKeys } from "../../commands/issueKeys";
import { type Item, itemGroupKey } from "./flatten";
import type { RowActionHandlers } from "./RowActions";

/** What the List's and Table's issue keys act on: this render's rows, selection and changes. */
export interface RowKeyState {
  items: Item[];
  rowIds: string[];
  selected: string | null;
  peekOpen: boolean;
  data: ViewData;
  select: (id: string, openPeek: boolean) => void;
  toggle: (group: Group) => void;
  setSubtasksOpen: (id: string, open: boolean) => void;
  subtasksOpen: (id: string) => boolean;
  actions: RowActionHandlers & { copyUrl: (key: string) => void };
  scrollToIndex: (index: number) => void;
  setPlace: (patch: Partial<ViewPlace>) => void;
  open: (id: string) => void;
}

/** Takes the issue keys for a List or Table while it is mounted. The keys reach the latest render's state
 *  through the returned ref, which callbacks that must stay stable can read too. */
export function useRowKeys(current: RowKeyState): RefObject<RowKeyState> {
  const state = useRef(current);
  useEffect(() => {
    state.current = current;
  });
  useEffect(() => {
    const withSelected = (run: (key: string) => void) => {
      const { data, selected } = state.current;
      const row = selected ? data.rows[selected] : undefined;
      if (row) run(row.key);
    };
    return onIssueKeys({
      move: (step) => {
        const { rowIds, selected, peekOpen, items, scrollToIndex, select } = state.current;
        if (rowIds.length === 0) return;
        const at = selected === null ? -1 : rowIds.indexOf(selected);
        const next = rowIds[Math.min(Math.max(at + step, 0), rowIds.length - 1)];
        select(next, peekOpen);
        scrollToIndex(items.findIndex((item) => item.kind === "row" && item.row.id === next));
      },
      setGroupOpen: (open) => {
        const { items, data, selected, toggle, select, peekOpen, setSubtasksOpen, subtasksOpen } = state.current;
        const row = items.find((item) => item.kind === "row" && item.row.id === selected);
        // In Hierarchy ←/→ on a row with children closes or opens its subtree, and ← on a nested row selects its
        // parent.
        if (row?.kind === "row" && row.nesting?.parent && subtasksOpen(row.row.id) !== open) {
          setSubtasksOpen(row.row.id, open);
          return;
        }
        if (row?.kind === "row" && row.nesting && row.nesting.depth > 0 && !open && row.row.parentId) {
          select(row.row.parentId, peekOpen);
          return;
        }
        // A selected row hidden in its closed group still opens it again.
        const groupKey = row ? itemGroupKey(row) : data.groups.find((g) => selected !== null && g.rowIds.includes(selected))?.key;
        const header = groupKey === undefined ? undefined : items.find((item) => item.kind === "group" && item.group.key === groupKey);
        if (header?.kind === "group" && header.open !== open) toggle(header.group);
      },
      closePeek: () => state.current.setPlace({ peekOpen: false }),
      copyKey: () => withSelected((key) => state.current.actions.copyKey(key)),
      copyUrl: () => withSelected((key) => state.current.actions.copyUrl(key)),
      openInJira: () => withSelected((key) => state.current.actions.openInJira?.(key)),
      open: () => {
        const { selected, open } = state.current;
        if (selected) open(selected);
      },
      openEpic: () => {
        const { data, selected, open } = state.current;
        const epic = selected ? data.rows[selected]?.epic : undefined;
        if (epic) open(epic.id);
      },
      togglePin: () => {
        const { selected, actions } = state.current;
        if (selected) actions.togglePin(selected);
      },
      toggleRead: () => {
        const { selected, actions, data } = state.current;
        if (!selected) return;
        if (data.rows[selected]?.changed) actions.markSeen(selected);
        else actions.markUnread(selected);
      },
      togglePeek: () => {
        const { selected, peekOpen, setPlace } = state.current;
        if (selected) setPlace({ peekOpen: !peekOpen });
      },
      hasSelection: () => state.current.selected !== null,
      peekOpen: () => state.current.peekOpen,
    });
  }, []);
  return state;
}
