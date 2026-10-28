import type { Group, Row, ViewData } from "../../api";

/** One line of the virtual list: a group header or an issue row. */
export type Item =
  | { kind: "group"; group: Group; open: boolean; first: boolean }
  | { kind: "row"; row: Row };

/** The groups as lines; a closed group contributes only its header. Without grouping there are no headers. */
export function flatten(data: ViewData, isOpen: (group: Group) => boolean): Item[] {
  const items: Item[] = [];
  data.groups.forEach((group, index) => {
    const open = data.groupBy === "none" || isOpen(group);
    if (data.groupBy !== "none") items.push({ kind: "group", group, open, first: index === 0 });
    if (!open) return;
    for (const id of group.rowIds) {
      const row = data.rows[id];
      if (row) items.push({ kind: "row", row });
    }
  });
  return items;
}

export const ROW_HEIGHT = 46;
export const GROUP_HEIGHT = 44;
/** Space above every group but the first. */
export const GROUP_GAP = 14;

export function itemHeight(item: Item): number {
  if (item.kind === "row") return ROW_HEIGHT;
  return item.first ? GROUP_HEIGHT : GROUP_HEIGHT + GROUP_GAP;
}
