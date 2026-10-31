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
import { DragList, useDragScroll } from "../list/DragList";
import { DropTray } from "../../writes/DropZone";
import { useIssueDrag } from "../../writes/useIssueDrag";

/** Every row, a group's heading row too: the Table never changes its rhythm. */
const TABLE_ROW_HEIGHT = 30;
const HEADER_HEIGHT = 34;
const SIDE_PADDING = 24;
const NO_HEADERS: string[] = [];
/** Rows in the drag layout neither select nor open. */
const NOTHING = () => {};

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
  const { toggled, collapsedParents, selected, setPlace, subtasksOpen } = place;
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
  const items = useMemo(
    () => flatten(data, (group: Group) => isOpen(group, toggled) || moving.has(group.key), collapsedParents).filter((item) => item.kind === "row" || item.tab),
    [data, toggled, moving, collapsedParents],
  );
  const rowIds = useMemo(() => items.flatMap((item) => (item.kind === "row" ? [item.row.id] : [])), [items]);
  const revealed = Object.keys(factors).length > 0 ? revealedHeights(items, factors, () => TABLE_ROW_HEIGHT) : null;
  const heightOf = (index: number) => revealed?.[index] ?? TABLE_ROW_HEIGHT;
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
  }, [factors, virtualizer]);

  const tree = data.subtasks === "hierarchy";
  const dropGroups = useMemo(() => data.groups.map((group) => ({ key: group.key, label: group.label })), [data.groups]);
  const { drag, onPointerDown } = useIssueDrag(data.groupBy, dropGroups, actions.openInJira, scrollRef, "y");
  // While an issue is dragged in a grouped view, every group shows with the same few rows (`DragList`).
  const dragging = drag !== null && data.groupBy !== "none";
  const dragLayout = useMemo(() => (dragging ? flatten(data, () => true).filter((item) => item.kind === "row" || item.tab) : null), [dragging, data]);
  useDragScroll(scrollRef, dragging ? (drag?.issue.fromKey ?? null) : null);
  const toggle = (group: Group) => {
    const opening = !isOpen(group, toggled);
    setPlace({ toggled: { ...toggled, [group.key]: opening } });
    animate(group.key, opening);
  };
  const select = (id: string, openPeek: boolean) => place.select(id, openPeek, rowIds);
  const open = (id: string) => openIssue(id, { listId: view.listId, viewId: view.id });
  const scrollToIndex = (index: number) => virtualizer.scrollToIndex(index, { align: "auto" });
  const keys = useRowKeys({ ...place, items, rowIds, data, select, toggle, actions, scrollToIndex, open });

  // Stable, so rows that didn't change skip re-rendering.
  const { onSelect, onOpen } = useRowClicks(keys, opening);
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
        {dragLayout && drag ? (
          <DragList
            drag={drag}
            items={dragLayout}
            variant="table"
            header={(group) => (
              <div className="table-item--group">
                <GroupHeader group={group} open tab active={false} namesColumns={false} headers={NO_HEADERS} onToggle={NOTHING} />
              </div>
            )}
            row={(item) => (
              <TableRow row={item.row} columns={columns} selected={false} onSelect={NOTHING} onOpen={NOTHING} inlineLabels={inlineLabels} tree={tree} nesting={item.nesting} />
            )}
          />
        ) : (
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
                  }}
                  onPointerDown={
                    item.kind === "row"
                      ? (e) => onPointerDown(e, { id: item.row.id, key: item.row.key, summary: item.row.summary, fromKey: item.groupKey }, e.currentTarget)
                      : undefined
                  }
                >
                  {item.kind === "group" ? (
                    <GroupHeader
                      group={item.group}
                      open={item.open}
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
                      selected={selected === item.row.id}
                      onSelect={onSelect}
                      onOpen={onOpen}
                      inlineLabels={inlineLabels}
                      tree={tree}
                      nesting={item.nesting}
                      subtasksOpen={subtasksOpen(item.row.id)}
                      onToggleSubtasks={onToggleSubtasks}
                      childChip={data.subtasks === "hidden" || (item.nesting?.parent === true && !subtasksOpen(item.row.id))}
                    />
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
      <DropTray drag={drag} groups={dropGroups} host={scrollRef} />
    </div>
  );
}
