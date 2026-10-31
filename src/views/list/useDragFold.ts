import { type RefObject, useLayoutEffect, useMemo, useRef } from "react";
import type { ViewData } from "../../api";
import type { Drag } from "../../writes/useIssueDrag";
import { foldItems, foldLayout, foldStart, type Fold } from "./dragFold";
import { flatten, type Item, itemKey } from "./flatten";
import { useFoldMotion } from "./useFoldMotion";

interface Options {
  data: ViewData;
  drag: Drag | null;
  /** The items the view shows when nothing is dragged. */
  shownItems: Item[];
  /** The groups open when nothing is dragged. */
  openGroups: ReadonlySet<string>;
  collapsedParents: readonly string[];
  scrollRef: RefObject<HTMLDivElement | null>;
  /** An item's height when shown whole. */
  height: (item: Item) => number;
  paddingTop: number;
  /** The `more` piece's own height below a group's rows. */
  morePx: number;
  /** Which items the view draws (the Table leaves out an ungrouped view's header). */
  keep?: (item: Item) => boolean;
}

/** A grouped List or Table folds every group to one height while an issue is dragged, and unfolds after. */
export function useDragFold({ data, drag, shownItems, openGroups, collapsedParents, scrollRef, height, paddingTop, morePx, keep }: Options): {
  items: Item[];
  fold: Fold | null;
} {
  const grouped = data.groupBy !== "none";
  const { progress, moving } = useFoldMotion(drag !== null && grouped);
  // The drag's issue, kept while the view unfolds after it.
  const lastDrag = useRef<Drag["issue"] | null>(null);
  if (drag !== null) lastDrag.current = drag.issue;
  const issue = lastDrag.current;
  const folding = moving && grouped && issue !== null;
  const items = useMemo(() => {
    if (!folding) return shownItems;
    const all = flatten(data, () => true, collapsedParents);
    return foldItems(keep ? all.filter(keep) : all);
  }, [folding, data, collapsedParents, shownItems, keep]);
  const draggedId = issue?.id ?? null;
  const fold = useMemo(() => {
    if (!folding || draggedId === null) return null;
    const full = (index: number) => height(items[index]);
    const normal = (index: number) => {
      const item = items[index];
      return item.kind === "row" && !openGroups.has(item.groupKey) ? 0 : full(index);
    };
    return foldLayout(items, progress, draggedId, full, normal, morePx);
  }, [folding, draggedId, items, progress, openGroups, height, morePx]);
  useFoldAnchor(items, fold, issue, scrollRef, paddingTop);
  return { items, fold };
}

/** Keeps the dragged row (or, once it moved to another group, its old group's header) where it is on screen. */
function useFoldAnchor(items: Item[], fold: Fold | null, issue: Drag["issue"] | null, scrollRef: RefObject<HTMLDivElement | null>, paddingTop: number) {
  const anchor = useRef<{ key: string; start: number } | null>(null);
  // After every render: any render during the fold may move the anchor.
  useLayoutEffect(() => {
    const view = scrollRef.current;
    if (!fold || !issue || !view) {
      anchor.current = null;
      return;
    }
    const index = anchorIndex(items, issue);
    if (index < 0) return;
    const key = itemKey(items[index]);
    const start = foldStart(fold.heights, index, paddingTop);
    const previous = anchor.current;
    if (previous?.key === key && previous.start !== start) view.scrollTop += start - previous.start;
    anchor.current = { key, start };
  });
}

/** The dragged row while it is in its group, else that group's header; -1 when neither shows. */
function anchorIndex(items: Item[], issue: Drag["issue"]): number {
  const row = items.findIndex((item) => item.kind === "row" && item.row.id === issue.id && item.groupKey === issue.fromKey);
  return row >= 0 ? row : items.findIndex((item) => item.kind === "group" && item.group.key === issue.fromKey);
}
