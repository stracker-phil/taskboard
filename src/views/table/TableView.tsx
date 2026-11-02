import { useVirtualizer } from "@tanstack/react-virtual";
import { useCallback, useLayoutEffect, useMemo, useRef } from "react";
import type { Group, Narrowing, View, ViewData } from "../../api";
import { useViewRows } from "../../queries/views";
import { useStore } from "../../state/store";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { flatten, itemKey, type Item } from "../list/flatten";
import { GroupHeader } from "../list/GroupHeader";
import { useGroupMotion } from "../list/useGroupMotion";
import { useIssueActions } from "../list/useIssueActions";
import { useOpenIssueInTab } from "../../shell/openInTab";
import { useRowClicks } from "../list/useRowClicks";
import { useRowKeys } from "../list/useRowKeys";
import { useRowsPlace } from "../list/useRowsPlace";
import { nextSort } from "./sortable";
import { TableHeader } from "./TableHeader";
import { TableRow } from "./TableRow";
import { useColumnResize } from "./useColumnResize";
import { useIssueOpening } from "../useIssueOpening";
import { useRememberedScroll } from "../useRememberedScroll";
import { useViewSettings } from "./useViewSettings";
import { columnWidth, SUMMARY_MIN_WIDTH, widthVar } from "./widths";
import { revealedHeights } from "../list/groupReveal";
import { foldSpans, type Fold } from "../list/dragFold";
import { useDragFold } from "../list/useDragFold";
import { BulkBar } from "../../writes/BulkBar";
import { DropTray, DropZone, zoneOf } from "../../writes/DropZone";
import { type Drag, useIssueDrag } from "../../writes/useIssueDrag";

/** Every row, a group's heading row too: the Table never changes its rhythm. */
const TABLE_ROW_HEIGHT = 30;
const HEADER_HEIGHT = 34;
const SIDE_PADDING = 24;
const NO_HEADERS: string[] = [];
/** The `more` piece's own height below a group's rows while an issue is dragged. */
const TABLE_MORE_PX = 28;
/** The Table's own header names the columns, so an ungrouped view draws no group header. */
const drawn = (item: Item) => item.kind !== "group" || item.tab;
const tableItemHeight = (item: Item) => (item.kind === "more" ? 0 : TABLE_ROW_HEIGHT);

const isOpen = (group: Group, toggled: Record<string, boolean>) => toggled[group.key] ?? !group.collapsedByDefault;

/** How far a row's group is open while it animates (1 when still). */
const openness = (item: Item, factors: Record<string, number>) => (item.kind === "row" ? (factors[item.groupKey] ?? 1) : 1);

interface Props {
  view: View;
  narrowing?: Narrowing;
}

/** The Table layout: a sticky, sortable, resizable header over the virtualized rows. */
export function TableView({ view, narrowing }: Props) {
  const { data } = useViewRows(view.listId, view.id, narrowing);
  if (!data) return null;
  if (Object.keys(data.rows).length === 0) return <EmptyLevel listId={view.listId} viewId={view.id} />;
  return <Table key={view.id} view={view} data={data} />;
}

/** Keyed by the view; its open groups, scroll, selection and peek come from the view's remembered place. */
function Table({ view, data }: { view: View; data: ViewData }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const place = useRowsPlace(view.listId, view.id, data);
  const { toggled, collapsedParents, selected, picked, setPlace, subtasksOpen } = place;
  const pickedIds = useMemo(() => new Set(picked), [picked]);
  const picking = picked.length > 0;
  const { factors, moving, animate } = useGroupMotion();
  const settings = useViewSettings(view);
  const resize = useColumnResize((id, width) => void settings.setWidth(id, width));
  const opening = useIssueOpening(view.openWith);
  const actions = useIssueActions();
  const openIssue = useStore((s) => s.openIssue);
  const onScroll = useRememberedScroll(view.listId, view.id, scrollRef);

  const widthOf = (id: string) => resize.liveWidth(id) ?? columnWidth(id, view.widths);
  // The Table always has its own Status column; a view's Status column would repeat it.
  const columns = useMemo(() => data.columns.filter((id) => id !== "status"), [data.columns]);
  const inlineLabels = view.inlineLabels && !data.columns.includes("labels");
  const headerColumns = ["key", "summary", "status", ...columns];
  const minWidth = headerColumns.reduce((sum, id) => sum + (id === "summary" ? SUMMARY_MIN_WIDTH : widthOf(id)), 2 * SIDE_PADDING);
  const widthVars = Object.fromEntries(headerColumns.filter((id) => id !== "summary").map((id) => [widthVar(id), `${widthOf(id)}px`]));

  // The Table's own header names the columns, so an ungrouped view needs no group header. A collapsing group
  // keeps its rows until they have shrunk away.
  const shownItems = useMemo(
    () => flatten(data, (group: Group) => isOpen(group, toggled) || moving.has(group.key), collapsedParents).filter(drawn),
    [data, toggled, moving, collapsedParents],
  );
  const dropGroups = useMemo(() => data.groups.map((group) => ({ key: group.key, label: group.label })), [data.groups]);
  const { drag, onPointerDown } = useIssueDrag(data.groupBy, dropGroups, actions.openInJira, scrollRef, "y");
  // While an issue is dragged in a grouped view, every group folds to the same height (`dragFold.ts`).
  const openGroups = useMemo(() => new Set(data.groups.filter((group) => isOpen(group, toggled)).map((group) => group.key)), [data.groups, toggled]);
  const { items, fold } = useDragFold({
    data,
    drag,
    shownItems,
    openGroups,
    collapsedParents,
    scrollRef,
    height: tableItemHeight,
    paddingTop: 0,
    morePx: TABLE_MORE_PX,
    keep: drawn,
  });
  const rowIds = useMemo(() => items.flatMap((item) => (item.kind === "row" ? [item.row.id] : [])), [items]);
  const revealed = Object.keys(factors).length > 0 ? revealedHeights(items, factors, () => TABLE_ROW_HEIGHT) : null;
  const heightOf = (index: number) => fold?.heights[index] ?? revealed?.[index] ?? TABLE_ROW_HEIGHT;
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: heightOf,
    getItemKey: (index) => itemKey(items[index]),
    scrollMargin: HEADER_HEIGHT,
    paddingEnd: 24,
    overscan: 12,
  });

  // A group in motion changes its rows' heights every frame; the virtualizer caches sizes.
  useLayoutEffect(() => {
    virtualizer.measure();
  }, [factors, fold, virtualizer]);

  const tree = data.subtasks === "hierarchy";
  const toggle = (group: Group) => {
    const opening = !isOpen(group, toggled);
    setPlace({ toggled: { ...toggled, [group.key]: opening } });
    animate(group.key, opening);
  };
  const select = (id: string, openPeek: boolean) => place.select(id, openPeek, rowIds);
  const open = (id: string) => openIssue(id, { listId: view.listId, viewId: view.id });
  const openIssueInTab = useOpenIssueInTab();
  const openInTab = (id: string, show: boolean) => void openIssueInTab(id, { listId: view.listId, viewId: view.id }, show);
  const scrollToIndex = (index: number) => virtualizer.scrollToIndex(index, { align: "auto" });
  const keys = useRowKeys({ ...place, items, rowIds, data, select, toggle, actions, scrollToIndex, open, openInTab });

  // Stable, so rows that didn't change skip re-rendering.
  const { onSelect, onOpen, onPick, onToggleBox } = useRowClicks(keys, opening);
  const onToggleSubtasks = useCallback((id: string) => keys.current.setSubtasksOpen(id, !keys.current.subtasksOpen(id)), [keys]);

  return (
    <div ref={scrollRef} className="table-scroll" role="grid" aria-rowcount={items.length} onScroll={onScroll}>
      <div className="table-content" style={{ minWidth, ...widthVars }}>
        <TableHeader
          listId={view.listId}
          columns={headerColumns}
          sort={view.sort}
          widthOf={widthOf}
          onSort={(by) => void settings.setSort(nextSort(view.sort, by))}
          onResizeStart={resize.start}
          tree={tree}
        />
        <div className="table-space" style={{ height: virtualizer.getTotalSize() }}>
          {virtualizer.getVirtualItems().map((virtual) => {
            const item = items[virtual.index];
            const factor = openness(item, factors);
            return (
              <div
                key={virtual.key}
                className={item.kind === "group" ? "table-item table-item--group" : "table-item"}
                style={{
                  height: virtual.size,
                  transform: `translateY(${virtual.start - HEADER_HEIGHT}px)`,
                  ...(factor < 1 && { opacity: factor, overflow: "hidden" }),
                  ...(fold && { opacity: fold.opacity[virtual.index], overflowY: "clip" }),
                }}
                onPointerDown={
                  item.kind === "row"
                    ? (e) => onPointerDown(e, { id: item.row.id, key: item.row.key, summary: item.row.summary, fromKey: item.groupKey }, e.currentTarget)
                    : undefined
                }
              >
                {item.kind === "more" ? (
                  <div className="table-more">{item.hidden > 0 ? `+${item.hidden} more` : ""}</div>
                ) : item.kind === "group" ? (
                  <GroupHeader
                    group={item.group}
                    open={item.open || (fold?.open.has(item.group.key) ?? false)}
                    tab
                    active
                    namesColumns={false}
                    headers={NO_HEADERS}
                    onToggle={() => toggle(item.group)}
                  />
                ) : (
                  <TableRow
                    row={item.row}
                    columns={columns}
                    selected={picking ? pickedIds.has(item.row.id) : selected === item.row.id}
                    picked={pickedIds.has(item.row.id)}
                    picking={picking}
                    onToggleBox={onToggleBox}
                    onSelect={onSelect}
                    onPick={onPick}
                    onOpen={onOpen}
                    inlineLabels={inlineLabels}
                    tree={tree}
                    nesting={item.nesting}
                    subtasksOpen={subtasksOpen(item.row.id)}
                    onToggleSubtasks={onToggleSubtasks}
                    childChip={data.subtasks === "hidden" || (item.nesting?.parent === true && !subtasksOpen(item.row.id))}
                    edits={actions}
                  />
                )}
              </div>
            );
          })}
          {drag && fold && <TableZones drag={drag} items={items} fold={fold} />}
        </div>
      </div>
      <DropTray drag={drag} groups={dropGroups} host={scrollRef} />
      <BulkBar picked={picked} onClear={() => place.setPicked([])} />
    </div>
  );
}

/** Each group's drop zone, laid exactly over its rows. */
function TableZones({ drag, items, fold }: { drag: Drag; items: Item[]; fold: Fold }) {
  return (
    <>
      {[...foldSpans(items, fold.heights, 0)].map(([key, span]) => {
        const zone = zoneOf(drag, key);
        return zone ? <DropZone key={key} zone={zone} className="drop-zone--rows" style={{ top: span.top, height: span.height }} /> : null;
      })}
    </>
  );
}
