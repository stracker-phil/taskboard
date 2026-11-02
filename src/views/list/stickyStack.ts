import type { Item } from "./flatten";
import { ancestorAt, depthOf, subtreeEnd } from "./hierarchy";

/** Where an item lies in the scroller, as the virtualizer measured it. */
export interface Span {
  start: number;
  end: number;
}

/**
 * A group header or a parent row that sticks while its group or subtree is in view. Its copy lies exactly over the
 * item and is CSS-sticky inside a track that spans the item and everything under it: it sticks at `offset` below the
 * scroller's top and is pushed out where the track ends, so it moves with the scroll itself and never lags or jumps.
 */
export interface Track {
  index: number;
  /** The track's top and height in the scroller: from the item's own top to the end of its group or subtree. */
  top: number;
  height: number;
  /** Where the copy sticks: below the group header and the ancestors that stick above it. */
  offset: number;
  /** -1 for a group header; deeper tracks are drawn first, so the outer ones cover them. */
  depth: number;
}

/** The tracks of the drawn items' group headers, ancestors and the drawn parents, but those scrolled past
 *  `scrollTop`. `heightAt` is an item's height while it sticks (a group's header without the gap above it). */
export function stickyTracks(items: Item[], spans: Span[], drawn: number[], scrollTop: number, heightAt: (index: number) => number): Track[] {
  const candidates = new Set<number>();
  for (const index of drawn) {
    const group = groupOf(items, index);
    if (group >= 0) candidates.add(group);
    for (const ancestor of ancestorsOf(items, index)) candidates.add(ancestor);
    const hasChildren = items[index].kind === "row" && subtreeEnd(items, index) > index;
    if (hasChildren) candidates.add(index);
  }
  const tracks: Track[] = [];
  for (const index of candidates) {
    const isGroup = items[index].kind === "group";
    const last = isGroup ? groupEnd(items, index) : subtreeEnd(items, index);
    const top = isGroup ? spans[index].end - heightAt(index) : spans[index].start;
    const height = spans[last].end - top;
    const offset = stackAbove(items, index, heightAt);
    if (top + height > scrollTop + offset) tracks.push({ index, top, height, offset, depth: depthOf(items[index]) });
  }
  return tracks.sort((a, b) => b.depth - a.depth || a.index - b.index);
}

/** How far below the top an item stays clear of the group header and ancestors that stick above it. */
export function stackAbove(items: Item[], index: number, heightAt: (index: number) => number): number {
  const group = groupOf(items, index);
  const header = group >= 0 && group !== index ? heightAt(group) : 0;
  return ancestorsOf(items, index).reduce((room, ancestor) => room + heightAt(ancestor), header);
}

/** A row's ancestors in its group, nearest first. */
function ancestorsOf(items: Item[], index: number): number[] {
  const found: number[] = [];
  let at = index;
  for (let depth = depthOf(items[index]) - 1; depth >= 0; depth--) {
    at = ancestorAt(items, at, depth);
    if (at < 0) break;
    found.push(at);
  }
  return found;
}

/** The index of the group header an item belongs to; -1 before the first. */
function groupOf(items: Item[], index: number): number {
  let at = index;
  while (at >= 0 && items[at].kind !== "group") at--;
  return at;
}

function groupEnd(items: Item[], group: number): number {
  let end = group;
  while (end + 1 < items.length && items[end + 1].kind !== "group") end++;
  return end;
}
