import type { Item } from "./flatten";
import { ancestorAt, depthOf, subtreeEnd } from "./hierarchy";

/** Where an item lies in the scroller, as the virtualizer measured it. */
export interface Span {
  start: number;
  end: number;
}

/** An item held at the top of the stack: `top` is its place below the stack's top edge. */
export interface StickyEntry {
  index: number;
  top: number;
  height: number;
}

interface Held extends StickyEntry {
  /** Where its group or subtree ends in the scroller; past it the entry is pushed up. */
  endsAt: number;
}

/** The group header and the ancestors still in view above the line `scrollTop` (where rows start to hide),
 *  outermost first, each pushed up as its group or subtree ends. `heightOf` is an item's height while held. */
export function stickyStack(items: Item[], spans: Span[], scrollTop: number, heightOf: (item: Item) => number): StickyEntry[] {
  const held: Held[] = [];
  let line = scrollTop;
  const hold = (index: number, endsAt: number) => {
    const height = heightOf(items[index]);
    held.push({ index, height, endsAt, top: line - scrollTop });
    line += height;
  };
  const first = firstEndingAfter(spans, line);
  if (first < 0) return [];
  const group = groupOf(items, first);
  // A group's item may start with the gap above it; only its header's own top counts.
  if (group >= 0 && spans[group].end - heightOf(items[group]) < line) hold(group, spans[groupEnd(items, group)].end);
  for (let depth = 0; ; depth++) {
    const at = firstEndingAfter(spans, line);
    // The row at the line must sit at this depth or under it; a shallower one has left the held subtree.
    if (at < 0 || depthOf(items[at]) < depth) break;
    const holder = depthOf(items[at]) === depth ? at : ancestorAt(items, at, depth);
    if (holder < 0 || spans[holder].start >= line) break;
    const end = subtreeEnd(items, holder);
    if (end === holder) break;
    hold(holder, spans[end].end);
  }
  return pushUp(held, scrollTop);
}

/** Each entry rises once its group or subtree ends; entries ending at the same row rise as one block, one ending
 *  sooner slides under the entry above. */
function pushUp(held: Held[], scrollTop: number): StickyEntry[] {
  for (const entry of held) entry.top = Math.min(entry.top, entry.endsAt - scrollTop - entry.height);
  for (let at = held.length - 2; at >= 0; at--) {
    if (held[at].endsAt === held[at + 1].endsAt) held[at].top = Math.min(held[at].top, held[at + 1].top - held[at].height);
  }
  return held.map(({ index, top, height }) => ({ index, top, height }));
}

/** How far below the top an item stays clear of the stack that holds its group and ancestors. */
export function stackAbove(items: Item[], index: number, heightOf: (item: Item) => number): number {
  const group = groupOf(items, index);
  let room = group >= 0 && group !== index ? heightOf(items[group]) : 0;
  let at = index;
  for (let depth = depthOf(items[index]) - 1; depth >= 0; depth--) {
    at = ancestorAt(items, at, depth);
    if (at < 0) break;
    room += heightOf(items[at]);
  }
  return room;
}

/** The index of the group header an item belongs to; -1 before the first. */
function groupOf(items: Item[], index: number): number {
  let at = index;
  while (at >= 0 && items[at].kind !== "group") at--;
  return at;
}

function firstEndingAfter(spans: Span[], line: number): number {
  let low = 0;
  let high = spans.length - 1;
  let found = -1;
  while (low <= high) {
    const mid = (low + high) >> 1;
    if (spans[mid].end > line) {
      found = mid;
      high = mid - 1;
    } else {
      low = mid + 1;
    }
  }
  return found;
}

function groupEnd(items: Item[], group: number): number {
  let end = group;
  while (end + 1 < items.length && items[end + 1].kind !== "group") end++;
  return end;
}
