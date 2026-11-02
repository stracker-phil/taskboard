import type { Item } from "../views/list/flatten";
import type { Span } from "../views/list/stickyStack";
import { group, row } from "./issues";

/**
 * Flattened list items from a compact outline: `#key` is a group header, any other entry a row named by its id,
 * nested one level per leading `-` (`"--Leaf"` sits at depth 2). Rows belong to the group above them.
 */
export function outline(...entries: string[]): Item[] {
  let groupKey = "g";
  return entries.map((entry, at): Item => {
    if (entry.startsWith("#")) {
      groupKey = entry.slice(1);
      return { kind: "group", group: group({ key: groupKey }), open: true, first: at === 0, tab: true };
    }
    const depth = entry.length - entry.replace(/^-+/, "").length;
    const id = entry.slice(depth);
    return { kind: "row", row: row({ id, depth }), groupKey, position: "middle", nesting: { depth, parent: false } };
  });
}

/** Consecutive spans, one per item, from the heights the items have while measured. */
export function spansOf(items: Item[], heightOf: (item: Item) => number): Span[] {
  let start = 0;
  return items.map((item) => {
    const span = { start, end: start + heightOf(item) };
    start = span.end;
    return span;
  });
}
