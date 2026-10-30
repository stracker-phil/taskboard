import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import type { ViewData, ViewPlace } from "../../api";
import { prefetchIssue, refreshIssueExtras } from "../../queries/issues";
import { narrowingKey } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";
import { isBelow } from "./flatten";

const NO_TOGGLED: Record<string, boolean> = {};
const NO_PARENTS: string[] = [];

/** Where the owner left a List or Table (open groups, collapsed parents, the selected row, its peek), loaded
 *  on mount, and the changes to it. */
export function useRowsPlace(listId: number, viewId: number, data: ViewData) {
  const key = narrowingKey(listId, viewId);
  const toggled = useStore((s) => s.places[key]?.toggled) ?? NO_TOGGLED;
  const collapsedParents = useStore((s) => s.places[key]?.collapsedParents) ?? NO_PARENTS;
  const selected = useStore((s) => s.places[key]?.selectedId ?? null);
  const peekOpen = useStore((s) => s.places[key]?.peekOpen ?? false);
  const loadPlace = useStore((s) => s.loadPlace);
  const updatePlace = useStore((s) => s.updatePlace);
  const client = useQueryClient();

  useEffect(() => {
    void loadPlace(listId, viewId);
  }, [loadPlace, listId, viewId]);

  const collapsed = useMemo(() => new Set(collapsedParents), [collapsedParents]);
  const setPlace = (patch: Partial<ViewPlace>) => updatePlace(listId, viewId, patch);
  const subtasksOpen = (id: string) => !collapsed.has(id);

  // Collapsing a row moves the selection from its hidden subtree onto it.
  const setSubtasksOpen = (id: string, open: boolean) => {
    setPlace({ collapsedParents: open ? collapsedParents.filter((p) => p !== id) : [...collapsedParents, id] });
    if (!open && selected && isBelow(data.rows, selected, id)) setPlace({ selectedId: id });
  };

  /** Selects the row and warms up its neighbors in `rowIds`, the rows in their shown order. */
  const select = (id: string, openPeek: boolean, rowIds: string[]) => {
    setPlace({ selectedId: id, peekOpen: openPeek });
    refreshIssueExtras(id);
    const at = rowIds.indexOf(id);
    for (const neighbor of [rowIds[at - 1], rowIds[at + 1]]) if (neighbor) prefetchIssue(client, neighbor);
  };

  return { toggled, collapsedParents, selected, peekOpen, setPlace, subtasksOpen, setSubtasksOpen, select };
}
