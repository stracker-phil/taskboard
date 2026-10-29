import { useEffect, useRef, useState } from "react";
import type { BoardData } from "../../api";
import { onIssueKeys } from "../../commands/issueKeys";
import type { RowActionHandlers } from "../list/RowActions";

/** The Board's issue keys: J/K (↓/↑) move through a column's cards, ←/→ to the neighboring open column, ↩
 *  opens; Copy, Bookmark, the read toggle and Open in Jira act on the current card. Returns the current card's id. */
export function useBoardKeys(data: BoardData, open: (id: string) => void, actions: RowActionHandlers & { copyUrl: (key: string) => void }) {
  const [selected, setSelected] = useState<string | null>(null);
  const state = useRef({ data, selected, open, actions });
  useEffect(() => {
    state.current = { data, selected, open, actions };
  });
  useEffect(() => {
    const columns = () => state.current.data.columns.filter((c) => !c.collapsed && c.ids.length > 0);
    const where = () => {
      const all = columns();
      const column = all.findIndex((c) => c.ids.includes(state.current.selected ?? ""));
      return { all, column, index: column >= 0 ? all[column].ids.indexOf(state.current.selected ?? "") : -1 };
    };
    const withKey = (run: (key: string) => void) => {
      const row = state.current.selected ? state.current.data.rows[state.current.selected] : undefined;
      if (row) run(row.key);
    };
    return onIssueKeys({
      move: (step) => {
        const { all, column, index } = where();
        if (all.length === 0) return;
        if (column < 0) return setSelected(all[0].ids[0]);
        const ids = all[column].ids;
        setSelected(ids[Math.min(Math.max(index + step, 0), ids.length - 1)]);
      },
      // ← and → go to the same place in the neighboring column.
      setGroupOpen: (right) => {
        const { all, column, index } = where();
        const next = all[column + (right ? 1 : -1)];
        if (next) setSelected(next.ids[Math.min(index, next.ids.length - 1)]);
      },
      closePeek: () => {},
      copyKey: () => withKey((key) => state.current.actions.copyKey(key)),
      copyUrl: () => withKey((key) => state.current.actions.copyUrl(key)),
      openInJira: () => withKey((key) => state.current.actions.openInJira?.(key)),
      open: () => {
        if (state.current.selected) state.current.open(state.current.selected);
      },
      openEpic: () => {
        const epic = state.current.selected ? state.current.data.rows[state.current.selected]?.epic : undefined;
        if (epic) state.current.open(epic.id);
      },
      togglePin: () => {
        if (state.current.selected) state.current.actions.togglePin(state.current.selected);
      },
      toggleRead: () => {
        const { selected, actions, data } = state.current;
        if (!selected) return;
        if (data.rows[selected]?.changed) actions.markSeen(selected);
        else actions.markUnread(selected);
      },
      togglePeek: () => {
        if (state.current.selected) state.current.open(state.current.selected);
      },
      hasSelection: () => state.current.selected !== null,
      // J/K start on the first card, so the Board takes them even before a card is current.
      peekOpen: () => false,
    });
  }, []);
  return selected;
}
