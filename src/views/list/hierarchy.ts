import type { Item } from "./flatten";

/** A guide's shape on one level: the parent's line passing by, a branch to a child with a later sibling, the curve
 *  to the last child, or nothing once the parent has no more children below. */
export const SHAPE = { pass: "|", branch: "+", last: "L", none: " " } as const;
/** How the path to the hovered or selected row lights a guide: as a straight line through the row, as the curved
 *  branch into a row on the path, or not at all. */
export const LIT = { through: "p", end: "e", off: "-" } as const;

/** Per nested row two characters per level above it, its shape and how the path lights it, as a string so a
 *  memoized row re-renders only when its own guides change. */
export function treeGuides(items: Item[], target: string | null): Map<string, string> {
  const later = hasLaterSibling(items);
  const guides = new Map<string, string[]>();
  const ancestors: number[] = [];
  items.forEach((item, index) => {
    const depth = depthOf(item);
    if (depth < 0) return;
    ancestors.length = depth;
    ancestors[depth] = index;
    if (item.kind !== "row" || depth === 0) return;
    const levels = Array.from({ length: depth }, (_, level) => {
      const branch = ancestors[level + 1];
      if (branch === index) return (later[index] ? SHAPE.branch : SHAPE.last) + LIT.off;
      return (later[branch] ? SHAPE.pass : SHAPE.none) + LIT.off;
    });
    guides.set(item.row.id, levels);
  });
  const at = target === null ? -1 : items.findIndex((item) => item.kind === "row" && item.row.id === target);
  if (at >= 0) lightPath(items, guides, at);
  return new Map([...guides].map(([id, levels]) => [id, levels.join("")]));
}

/** Whether each row has a sibling after it: a row at its depth before anything shallower. */
function hasLaterSibling(items: Item[]): boolean[] {
  const later: boolean[] = [];
  const seen: boolean[] = [];
  for (let at = items.length - 1; at >= 0; at--) {
    const depth = depthOf(items[at]);
    if (depth < 0) {
      seen.length = 0;
      continue;
    }
    later[at] = seen[depth] === true;
    seen.length = depth + 1;
    seen[depth] = true;
  }
  return later;
}

/** Each level's line from the ancestor down to the next row on the path, curving into it. */
function lightPath(items: Item[], guides: Map<string, string[]>, target: number) {
  let node = target;
  for (let level = depthOf(items[target]) - 1; level >= 0; level--) {
    const ancestor = ancestorAt(items, node, level);
    if (ancestor < 0) return;
    for (let at = ancestor + 1; at <= node; at++) {
      const item = items[at];
      const levels = item.kind === "row" ? guides.get(item.row.id) : undefined;
      if (levels) levels[level] = levels[level][0] + (at === node ? LIT.end : LIT.through);
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
