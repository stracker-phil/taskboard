import type { Virtualizer } from "@tanstack/react-virtual";
import type { Item } from "./flatten";
import { stackAbove, stickyTracks } from "./stickyStack";

interface Options {
  /** What sits above the rows at the scroller's top: the Table's own header. */
  inset: number;
  /** An item's height while it sticks. */
  heightAt: (index: number) => number;
  /** Off while an issue is dragged, when the items don't sit where they were measured. */
  held: boolean;
}

/** The sticky tracks of a List or Table (`stickyTracks`), and a scroll that keeps an item clear of what sticks above
 *  it. Nothing sticks at the very top, so an unscrolled view draws no copies. */
export function useStickyStack(items: Item[], virtualizer: Virtualizer<HTMLDivElement, Element>, { inset, heightAt, held }: Options) {
  const scrollTop = virtualizer.scrollOffset ?? 0;
  const spans = virtualizer.measurementsCache;
  const drawn = virtualizer.getVirtualItems().map((virtual) => virtual.index);
  const tracks = held && scrollTop > 0 && spans.length === items.length ? stickyTracks(items, spans, drawn, scrollTop + inset, heightAt) : [];
  /** Scrolls the item into view right below what sticks above it. */
  const reveal = (index: number) => {
    const span = spans[index];
    if (!span) return;
    const offset = span.start - inset - stackAbove(items, index, heightAt);
    if (offset < scrollTop) virtualizer.scrollToOffset(offset);
    else virtualizer.scrollToIndex(index, { align: "auto" });
  };
  return { tracks, reveal };
}

/** A row of a group that is opening or closing: it fades and is cut, so its copy would not match it. */
export const inMotion = (item: Item, factors: Record<string, number>) => item.kind === "row" && item.groupKey in factors;
