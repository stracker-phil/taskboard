import { useVirtualizer, type VirtualItem } from "@tanstack/react-virtual";
import { type CSSProperties, type RefObject, useLayoutEffect, useRef } from "react";
import { type Item, itemHeight, itemKey } from "./flatten";
import type { Fold } from "./dragFold";
import { revealedHeights } from "./groupReveal";

/** Space above the first group. */
export const LIST_PADDING_TOP = 22;

/** How far a row's group is open while it animates (1 when still). */
function motionFactor(item: Item, factors: Record<string, number>): number {
  return item.kind === "row" ? (factors[item.groupKey] ?? 1) : 1;
}

/**
 * The virtualizer and each item's size. Fixed-height rows take their height from the item; wrapped rows are
 * measured. While a group opens or closes its rows keep their height, fade, and are cut at the group's moving
 * bottom edge (`revealedHeights`); a wrapped row keeps the height it had when the motion started. While an issue is
 * dragged the `fold` sets every item's height and a row's opacity.
 */
export function useRowSizing(items: Item[], factors: Record<string, number>, wrapTitles: boolean, scrollRef: RefObject<HTMLDivElement | null>, fold: Fold | null = null) {
  const startHeights = useRef(new Map<string, number>());
  const fullHeight = (index: number) => {
    const item = items[index];
    return (item.kind === "row" && wrapTitles ? startHeights.current.get(item.row.id) : undefined) ?? itemHeight(item);
  };
  const revealed = Object.keys(factors).length > 0 ? revealedHeights(items, factors, fullHeight) : null;
  const heightOf = (index: number) => fold?.heights[index] ?? revealed?.[index] ?? itemHeight(items[index]);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: heightOf,
    getItemKey: (index) => itemKey(items[index]),
    // An element that reports no height yet keeps its estimate.
    measureElement: (element, _entry, instance) =>
      Math.round(element.getBoundingClientRect().height) || heightOf(instance.indexFromElement(element)),
    paddingStart: LIST_PADDING_TOP,
    paddingEnd: 40,
    overscan: 12,
  });

  // Fixed sizes are cached, and a group in motion changes them every frame; measured rows report their own.
  useLayoutEffect(() => {
    if (!wrapTitles || fold) virtualizer.measure();
  }, [factors, fold, virtualizer, wrapTitles]);

  const groupStartsMoving = (groupKey: string) => {
    if (!wrapTitles) return;
    items.forEach((item, index) => {
      const size = virtualizer.measurementsCache[index]?.size;
      if (item.kind === "row" && item.groupKey === groupKey && size) startHeights.current.set(item.row.id, size);
    });
  };

  const itemProps = (virtual: VirtualItem, item: Item) => {
    const style: CSSProperties = { transform: `translateY(${virtual.start}px)` };
    if (fold) {
      Object.assign(style, { height: fold.heights[virtual.index], opacity: fold.opacity[virtual.index], overflowY: "clip" });
      return { "data-index": virtual.index, style };
    }
    if (!wrapTitles) style.height = virtual.size;
    if (item.kind === "row" && item.groupKey in factors) {
      // Clip only vertically: a top-level row's caret sits in the gutter left of the card.
      Object.assign(style, { opacity: motionFactor(item, factors), overflowY: "clip", height: heightOf(virtual.index) });
    }
    return { ref: wrapTitles ? virtualizer.measureElement : undefined, "data-index": virtual.index, style };
  };

  return { virtualizer, groupStartsMoving, itemProps };
}
