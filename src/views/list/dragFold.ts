import type { CardPosition, Item } from "./flatten";

/** Rows every group keeps while an issue is dragged; below them the card's "+N more" piece. */
export const FOLD_ROWS = 2;

/** The view while an issue is dragged: every group open, with a `more` piece closing each group's card. */
export function foldItems(items: Item[]): Item[] {
  const folded: Item[] = [];
  let open: Extract<Item, { kind: "group" }> | null = null;
  let rows = 0;
  const close = () => {
    if (open) folded.push({ kind: "more", groupKey: open.group.key, tab: open.tab, hidden: Math.max(0, rows - FOLD_ROWS) });
  };
  for (const item of items) {
    if (item.kind === "group") {
      close();
      open = item;
      rows = 0;
    } else if (item.kind === "row") {
      rows++;
    }
    folded.push(item);
  }
  close();
  return folded;
}

/** How each item of `foldItems` shows at `progress` (0: as the view normally is, 1: folded). */
export interface Fold {
  heights: number[];
  /** Rows that fold away fade out, rows of closed groups fade in. */
  opacity: number[];
  /** Card corners while the `more` piece is tall enough to close the card. */
  positions: Map<number, CardPosition>;
  /** Groups whose header shows open (a closed group's tab opens as its rows appear). */
  open: Set<string>;
}

/** A card piece needs this much height before it takes the card's rounded corners. */
const CORNER_PX = 10;

/**
 * Folded, each group's card is the same height: its first `FOLD_ROWS` rows (the dragged row always among them,
 * keeping its place) and the `more` piece filling up to `FOLD_ROWS` rows plus `morePx`. Every item's height moves
 * linearly from how the view normally shows it (`normal`: a closed group's rows 0) to its folded height, so groups
 * shrink or grow and the groups below glide.
 */
export function foldLayout(
  items: Item[],
  progress: number,
  draggedId: string,
  full: (index: number) => number,
  normal: (index: number) => number,
  morePx: number,
): Fold {
  const kept = keptRows(items, draggedId);
  const folded = items.map((item, index) => (item.kind === "row" ? (kept.has(index) ? full(index) : 0) : full(index)));
  let rowsAbove = 0;
  let rowHeight = 0;
  items.forEach((item, index) => {
    if (item.kind === "group") {
      rowsAbove = 0;
      rowHeight = 0;
    }
    if (item.kind === "row") {
      rowsAbove += folded[index];
      rowHeight = rowHeight || full(index);
    }
    if (item.kind === "more") folded[index] = FOLD_ROWS * (rowHeight || morePx) + morePx - rowsAbove;
  });
  const heights = items.map((_, index) => normal(index) + (folded[index] - normal(index)) * progress);
  const opacity = items.map((item, index) => {
    if (item.kind !== "row") return 1;
    if (folded[index] === 0) return 1 - progress;
    return normal(index) === 0 ? progress : 1;
  });
  return { heights, opacity, positions: positions(items, heights), open: openGroups(items, heights) };
}

/** The rows each group keeps: its first `FOLD_ROWS`, the last of them giving way to the dragged row. */
function keptRows(items: Item[], draggedId: string): Set<number> {
  const kept = new Set<number>();
  let group: number[] = [];
  let dragged: number | null = null;
  const settle = () => {
    const first = group.slice(0, FOLD_ROWS);
    if (dragged !== null && !first.includes(dragged)) first[FOLD_ROWS - 1] = dragged;
    first.forEach((index) => kept.add(index));
    group = [];
    dragged = null;
  };
  items.forEach((item, index) => {
    if (item.kind !== "row") return settle();
    group.push(index);
    if (item.row.id === draggedId) dragged = index;
  });
  settle();
  return kept;
}

/** Once a group's `more` piece is tall enough to round the card's bottom, its rows above stay square. */
function positions(items: Item[], heights: number[]): Map<number, CardPosition> {
  const result = new Map<number, CardPosition>();
  let rows: number[] = [];
  items.forEach((item, index) => {
    if (item.kind === "group") rows = [];
    if (item.kind === "row") rows.push(index);
    if (item.kind === "more" && heights[index] >= CORNER_PX) rows.forEach((row, at) => result.set(row, at === 0 ? "first" : "middle"));
  });
  return result;
}

function openGroups(items: Item[], heights: number[]): Set<string> {
  const open = new Set<string>();
  items.forEach((item, index) => {
    if (item.kind === "row" && heights[index] > 0) open.add(item.groupKey);
  });
  return open;
}

/** Where a group's rows and `more` piece sit in the scroll space, below its header: the group's drop zone. */
export interface DropSpan {
  top: number;
  height: number;
  /** The group has a tab, which squares the card's top-left corner. */
  tab: boolean;
}

export function foldSpans(items: Item[], heights: number[], paddingStart: number): Map<string, DropSpan> {
  const spans = new Map<string, DropSpan>();
  let at = paddingStart;
  let top: number | null = null;
  items.forEach((item, index) => {
    if (item.kind === "group") top = null;
    else if (top === null) top = at;
    at += heights[index];
    if (item.kind === "more" && top !== null) spans.set(item.groupKey, { top, height: at - top, tab: item.tab });
  });
  return spans;
}

export function foldStart(heights: number[], index: number, paddingStart: number): number {
  let at = paddingStart;
  for (let i = 0; i < index; i++) at += heights[i];
  return at;
}
