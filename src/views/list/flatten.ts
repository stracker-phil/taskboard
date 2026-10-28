import type { Group, Row, ViewData } from "../../api";

/** Where a row sits in its group's card, which decides its corners and edges. */
export type CardPosition = "first" | "middle" | "last" | "only";

/** One line of the virtual list: a group's header (tab and column headers) or an issue row. An ungrouped
 *  view has one header without a tab. */
export type Item =
  | { kind: "group"; group: Group; open: boolean; first: boolean; tab: boolean }
  | { kind: "row"; row: Row; groupKey: string; position: CardPosition };

export function flatten(data: ViewData, isOpen: (group: Group) => boolean): Item[] {
  const items: Item[] = [];
  const grouped = data.groupBy !== "none";
  data.groups.forEach((group, index) => {
    const open = !grouped || isOpen(group);
    items.push({ kind: "group", group, open, first: index === 0, tab: grouped });
    if (!open) return;
    const rows = group.rowIds.flatMap((id) => data.rows[id] ?? []);
    rows.forEach((row, at) => {
      items.push({ kind: "row", row, groupKey: group.key, position: position(at, rows.length) });
    });
  });
  return items;
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
