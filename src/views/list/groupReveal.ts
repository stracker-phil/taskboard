import type { Item } from "./flatten";

/**
 * The height each item shows while groups open or close: a moving group reveals the top `factor` of its rows'
 * total height, so its rows keep their own height and appear or vanish at the group's bottom edge, as the
 * sidebar's Spaces do. Items of still groups show whole. `full` is an item's height when shown whole.
 */
export function revealedHeights(items: Item[], factors: Record<string, number>, full: (index: number) => number): number[] {
  const totals = new Map<string, number>();
  items.forEach((item, index) => {
    if (item.kind === "row" && item.groupKey in factors) totals.set(item.groupKey, (totals.get(item.groupKey) ?? 0) + full(index));
  });
  const above = new Map<string, number>();
  return items.map((item, index) => {
    const height = full(index);
    if (item.kind !== "row" || !(item.groupKey in factors)) return height;
    const offset = above.get(item.groupKey) ?? 0;
    above.set(item.groupKey, offset + height);
    const shown = (totals.get(item.groupKey) ?? 0) * factors[item.groupKey];
    return Math.max(0, Math.min(height, shown - offset));
  });
}
