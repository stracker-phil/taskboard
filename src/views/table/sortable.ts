import type { Sort, SortBy } from "../../api";

const SORTABLE: Record<string, SortBy> = { key: "key", summary: "summary", status: "status", priority: "priority", due: "due" };
const DESCENDING_FIRST: SortBy[] = ["updated", "created", "priority", "due"];

/** The sort a column header stands for, or null when the column cannot sort. */
export function sortByOf(columnId: string): SortBy | null {
  return SORTABLE[columnId] ?? null;
}

/** Clicking the sorted column flips its direction; another column starts in its natural one. */
export function nextSort(current: Sort, by: SortBy): Sort {
  if (current.by === by) return { by, descending: !current.descending };
  return { by, descending: DESCENDING_FIRST.includes(by) };
}
