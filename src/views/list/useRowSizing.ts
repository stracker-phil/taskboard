import { useVirtualizer, type VirtualItem } from "@tanstack/react-virtual";
import { type CSSProperties, type RefObject, useLayoutEffect, useRef } from "react";
import { type Item, itemHeight, itemKey } from "./flatten";

/** How far a row's group is open while it animates (1 when still). */
function motionFactor(item: Item, factors: Record<string, number>): number {
  return item.kind === "row" ? (factors[item.groupKey] ?? 1) : 1;
}

/**
 * The virtualizer and each item's size. Fixed-height rows take their height from the item; wrapped rows are
 * measured and, while their group collapses, shrink from the height they had when it started.
 */
export function useRowSizing(items: Item[], factors: Record<string, number>, wrapTitles: boolean, scrollRef: RefObject<HTMLDivElement | null>) {
  const heightOf = (index: number) => itemHeight(items[index]) * motionFactor(items[index], factors);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: heightOf,
    getItemKey: (index) => itemKey(items[index]),
    // An element that reports no height yet keeps its estimate.
    measureElement: (element, _entry, instance) =>
      Math.round(element.getBoundingClientRect().height) || heightOf(instance.indexFromElement(element)),
    paddingStart: 22,
    paddingEnd: 40,
    overscan: 12,
  });

  // Fixed sizes are cached, and a group in motion changes them every frame; measured rows report their own.
  useLayoutEffect(() => {
    if (!wrapTitles) virtualizer.measure();
  }, [factors, virtualizer, wrapTitles]);

  const startHeights = useRef(new Map<string, number>());
  const groupStartsMoving = (groupKey: string) => {
    if (!wrapTitles) return;
    items.forEach((item, index) => {
      const size = virtualizer.measurementsCache[index]?.size;
      if (item.kind === "row" && item.groupKey === groupKey && size) startHeights.current.set(item.row.id, size);
    });
  };

  const itemProps = (virtual: VirtualItem, item: Item) => {
    const style: CSSProperties = { transform: `translateY(${virtual.start}px)` };
    if (!wrapTitles) style.height = virtual.size;
    if (item.kind === "row" && item.groupKey in factors) {
      const factor = motionFactor(item, factors);
      const start = wrapTitles ? startHeights.current.get(item.row.id) : undefined;
      // Clip only vertically: a top-level row's caret sits in the gutter left of the card.
      Object.assign(style, { opacity: factor, overflowY: "clip" }, start === undefined ? {} : { height: start * factor });
    }
    return { ref: wrapTitles ? virtualizer.measureElement : undefined, "data-index": virtual.index, style };
  };

  return { virtualizer, groupStartsMoving, itemProps };
}
