import { itemGroupKey, type Item } from "./flatten";

/** Where a group's drop zone sits in the list's scroll space. */
export interface Span {
  top: number;
  height: number;
  /** A closed group: the zone covers its header row only. */
  compact: boolean;
}

/** How far a zone reaches into the gap above the next group's tab, so the space between groups takes drops too. */
const GAP_REACH = 28;
/** The header row's height (`.list-group`). */
const HEADER_PX = 30;

/** Each group's zone: from its first row (its header stays clear) down into the gap before the next group; a
 *  closed group's zone is its header row. `measured[i]` is where item i sits; `total` the scroll space's height. */
export function groupSpans(items: Item[], measured: { start: number; end: number }[], total: number): Map<string, Span> {
  const spans = new Map<string, Span>();
  let first = 0;
  while (first < items.length) {
    const key = itemGroupKey(items[first]);
    let last = first;
    while (last + 1 < items.length && itemGroupKey(items[last + 1]) === key) last++;
    const hasHeader = items[first].kind === "group";
    const firstRow = hasHeader ? first + 1 : first;
    const compact = firstRow > last;
    const top = compact ? measured[first].end - HEADER_PX : measured[firstRow].start;
    const bottom = Math.min(total, measured[last].end + (last + 1 < items.length ? GAP_REACH : 0));
    spans.set(key, { top, height: Math.max(0, bottom - top), compact });
    first = last + 1;
  }
  return spans;
}
