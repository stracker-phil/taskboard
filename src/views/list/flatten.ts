import type { Group, Row, ViewData } from "../../api";

/** Where a row sits in its group's card, which decides its corners and edges. */
export type CardPosition = "first" | "middle" | "last" | "only";

/** A row's place in Hierarchy's tree (ADR 0040): how deep it nests and whether it has children to collapse;
 *  null in the other modes. */
export type Nesting = { depth: number; parent: boolean } | null;

/** One line of the virtual list: a group's header (tab and column headers) or an issue row. An ungrouped
 *  view has one header without a tab. */
export type Item =
  | { kind: "group"; group: Group; open: boolean; first: boolean; tab: boolean }
  | { kind: "row"; row: Row; groupKey: string; position: CardPosition; nesting: Nesting };

/** The rows of open groups; in Hierarchy without the subtrees of `collapsedParents`. */
export function flatten(data: ViewData, isOpen: (group: Group) => boolean, collapsedParents: readonly string[] = []): Item[] {
  const items: Item[] = [];
  const grouped = data.groupBy !== "none";
  const hierarchy = data.subtasks === "hierarchy";
  data.groups.forEach((group, index) => {
    const open = !grouped || isOpen(group);
    items.push({ kind: "group", group, open, first: index === 0, tab: grouped });
    if (!open) return;
    const all = group.rowIds.flatMap((id) => data.rows[id] ?? []);
    const shown: { row: Row; nesting: Nesting }[] = [];
    // Rows deeper than a collapsed row belong to its subtree until one comes back up to its depth.
    let collapsedAt: number | null = null;
    all.forEach((row, at) => {
      if (!hierarchy) {
        shown.push({ row, nesting: null });
        return;
      }
      if (collapsedAt !== null && row.depth > collapsedAt) return;
      collapsedAt = null;
      const parent = (all[at + 1]?.depth ?? 0) > row.depth;
      if (parent && collapsedParents.includes(row.id)) collapsedAt = row.depth;
      shown.push({ row, nesting: { depth: row.depth, parent } });
    });
    shown.forEach(({ row, nesting }, at) => {
      items.push({ kind: "row", row, groupKey: group.key, position: position(at, shown.length), nesting });
    });
  });
  return items;
}

/** Whether `id` sits somewhere below `ancestor`, following the rows' parents. */
export function isBelow(rows: ViewData["rows"], id: string, ancestor: string): boolean {
  const seen = new Set<string>();
  for (let at = rows[id]?.parentId; at && !seen.has(at); at = rows[at]?.parentId) {
    if (at === ancestor) return true;
    seen.add(at);
  }
  return false;
}

function position(index: number, count: number): CardPosition {
  if (count === 1) return "only";
  if (index === 0) return "first";
  return index === count - 1 ? "last" : "middle";
}

export const ROW_HEIGHT = 44;
/** The tab row above each card. */
export const GROUP_HEIGHT = 30;
/** Space above every group but the first: one row. */
export const GROUP_GAP = 44;

/** Stable across refreshes, so a measured height stays with its row when rows come and go. */
export function itemKey(item: Item): string {
  return item.kind === "row" ? `row:${item.row.id}` : `group:${item.group.key}`;
}

export function itemHeight(item: Item): number {
  if (item.kind === "row") return ROW_HEIGHT;
  return item.first ? GROUP_HEIGHT : GROUP_HEIGHT + GROUP_GAP;
}

export function itemGroupKey(item: Item): string {
  return item.kind === "group" ? item.group.key : item.groupKey;
}
