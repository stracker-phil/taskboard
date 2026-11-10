/** How clicks and keys change the picked issues of a view. `rowIds` are the view's rows in the order they show;
 *  every result keeps that order. */

const inOrder = (rowIds: string[], ids: ReadonlySet<string>) => rowIds.filter((id) => ids.has(id));

/** ⌘-click: adds or removes the row. The first one also picks the row that was selected, as Finder does. */
export function togglePick(rowIds: string[], picked: string[], id: string, selected: string | null): string[] {
  const next = new Set(picked.length === 0 && selected !== null ? [selected] : picked);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return inOrder(rowIds, next);
}

/** A checkbox: adds or removes that row alone. */
export function toggleBox(rowIds: string[], picked: string[], id: string): string[] {
  const next = new Set(picked);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  return inOrder(rowIds, next);
}

/** ⇧-click: the rows from the selected one to the clicked one, both included. */
export function pickRange(rowIds: string[], selected: string | null, id: string): string[] {
  const to = rowIds.indexOf(id);
  const from = selected === null ? -1 : rowIds.indexOf(selected);
  if (to < 0) return [];
  if (from < 0) return [id];
  return rowIds.slice(Math.min(from, to), Math.max(from, to) + 1);
}

/** ⇧J / ⇧K: the picked rows plus the next one past their bottom (⇧J) or top (⇧K) edge, or past the selected row
 *  when none is picked; returns the new picks and the row that is selected now. */
export function extendPick(rowIds: string[], picked: string[], selected: string | null, step: 1 | -1): { picked: string[]; selected: string | null } {
  if (rowIds.length === 0) return { picked, selected };
  const ids = new Set(picked.length === 0 && selected !== null ? [selected] : picked);
  const at = ids.size === 0 ? -1 : step > 0 ? Math.max(...[...ids].map((id) => rowIds.indexOf(id))) : Math.min(...[...ids].map((id) => rowIds.indexOf(id)));
  const next = rowIds[Math.min(Math.max(at + step, 0), rowIds.length - 1)];
  ids.add(next);
  return { picked: inOrder(rowIds, ids), selected: next };
}

/** A group's checkbox: unpicks the group's rows when every one is picked, otherwise picks them all; other picks
 *  stay. */
export function toggleGroup(rowIds: string[], picked: string[], groupIds: string[]): string[] {
  const next = new Set(picked);
  const all = groupIds.length > 0 && groupIds.every((id) => next.has(id));
  for (const id of groupIds) {
    if (all) next.delete(id);
    else next.add(id);
  }
  return inOrder(rowIds, next);
}

/** How many of a group's rows are picked, as its checkbox shows it. */
export type GroupPicks = "all" | "some" | "none";

export function groupPicks(groupIds: string[], picked: ReadonlySet<string>): GroupPicks {
  const count = groupIds.filter((id) => picked.has(id)).length;
  if (count === 0) return "none";
  return count === groupIds.length ? "all" : "some";
}

/** Every row of a List or Table in its order, the rows of collapsed groups and subtrees included, each once. */
export function viewOrder(groups: { rowIds: string[] }[], rows: Record<string, unknown>): string[] {
  return [...new Set(groups.flatMap((group) => group.rowIds))].filter((id) => id in rows);
}

/** The rows a group's checkbox picks: all of the group's, those of a collapsed group or subtree included. */
export function groupRowIds(group: { rowIds: string[] }, rows: Record<string, unknown>): string[] {
  return group.rowIds.filter((id) => id in rows);
}
