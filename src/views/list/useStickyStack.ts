import type { Virtualizer } from "@tanstack/react-virtual";
import type { Item } from "./flatten";
import { stackAbove, stickyStack } from "./stickyStack";

interface Options {
  /** What sits above the rows at the scroller's top: the Table's own header. */
  inset: number;
  /** An item's height while the stack holds it. */
  heightOf: (item: Item) => number;
  /** Off while an issue is dragged. */
  held: boolean;
}

/** The group header and ancestors held at the top of a scrolled List or Table, and a scroll that keeps an item
 *  clear of them. */
export function useStickyStack(items: Item[], virtualizer: Virtualizer<HTMLDivElement, Element>, { inset, heightOf, held }: Options) {
  const scrollTop = virtualizer.scrollOffset ?? 0;
  const entries = held ? stickyStack(items, virtualizer.measurementsCache, scrollTop + inset, heightOf) : [];
  /** Scrolls the item into view right below what the stack holds above it; "start" puts it there even when visible. */
  const reveal = (index: number, align: "auto" | "start" = "auto") => {
    const span = virtualizer.measurementsCache[index];
    if (!span) return;
    const offset = span.start - inset - stackAbove(items, index, heightOf);
    if (align === "start" || offset < scrollTop) virtualizer.scrollToOffset(offset);
    else virtualizer.scrollToIndex(index, { align: "auto" });
  };
  return { entries, reveal };
}
