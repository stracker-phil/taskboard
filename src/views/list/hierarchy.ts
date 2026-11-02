import type { Item } from "./flatten";

/** Indent-guide marks: a faint line, the highlighted path, or the path's end. */
export const GUIDE = { line: "l", path: "p", end: "e" } as const;

/** Indent marks per nested row, one per level above it, as a string so a memoized row re-renders only when its own
 *  marks change. The path runs from `target` up to its top-level ancestor. */
export function treeGuides(items: Item[], target: string | null): Map<string, string> {
  const marks = new Map<string, string[]>();
  for (const item of items) {
    const depth = depthOf(item);
    if (item.kind === "row" && depth > 0) marks.set(item.row.id, Array<string>(depth).fill(GUIDE.line));
  }
  const at = target === null ? -1 : items.findIndex((item) => item.kind === "row" && item.row.id === target);
  if (at >= 0) markPath(items, marks, at);
  return new Map([...marks].map(([id, row]) => [id, row.join("")]));
}

/** Each level's line from the ancestor down to the next row on the path, ending at that row. */
function markPath(items: Item[], marks: Map<string, string[]>, target: number) {
  let node = target;
  for (let level = depthOf(items[target]) - 1; level >= 0; level--) {
    const ancestor = ancestorAt(items, node, level);
    if (ancestor < 0) return;
    for (let at = ancestor + 1; at <= node; at++) {
      const item = items[at];
      const row = item.kind === "row" ? marks.get(item.row.id) : undefined;
      if (row) row[level] = at === node ? GUIDE.end : GUIDE.path;
    }
    node = ancestor;
  }
}

/** A row's depth in Hierarchy; 0 outside it, -1 for anything but a row. */
export function depthOf(item: Item | undefined): number {
  return item?.kind === "row" ? (item.nesting?.depth ?? 0) : -1;
}

/** The nearest row above `from` at `depth`, which holds it in its subtree; -1 when its group starts first. */
export function ancestorAt(items: Item[], from: number, depth: number): number {
  for (let at = from - 1; at >= 0; at--) {
    const found = depthOf(items[at]);
    if (found === depth) return at;
    if (found < depth) return -1;
  }
  return -1;
}

/** The last item of the subtree under the row at `index` (the row itself when nothing nests under it). */
export function subtreeEnd(items: Item[], index: number): number {
  const depth = depthOf(items[index]);
  let end = index;
  while (end + 1 < items.length && depthOf(items[end + 1]) > depth) end++;
  return end;
}
