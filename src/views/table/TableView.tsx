import { useVirtualizer } from "@tanstack/react-virtual";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Group, Narrowing, View, ViewData } from "../../api";
import { useViewRows } from "../../queries/views";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { flatten, itemKey, type Item } from "../list/flatten";
import { GroupHeader } from "../list/GroupHeader";
import { useGroupMotion } from "../list/useGroupMotion";
import { nextSort } from "./sortable";
import { TableHeader } from "./TableHeader";
import { TableRow } from "./TableRow";
import { useColumnResize } from "./useColumnResize";
import { useViewSettings } from "./useViewSettings";
import { columnWidth, SUMMARY_MIN_WIDTH } from "./widths";

/** Every row, a group's heading row too: the Table never changes its rhythm. */
const TABLE_ROW_HEIGHT = 30;
const HEADER_HEIGHT = 34;
const SIDE_PADDING = 24;

const isOpen = (group: Group, toggled: Record<string, boolean>) => toggled[group.key] ?? !group.collapsedByDefault;

/** How far a row's group is open while it animates (1 when still). */
const openness = (item: Item, factors: Record<string, number>) => (item.kind === "row" ? (factors[item.groupKey] ?? 1) : 1);

interface Props {
  view: View;
  narrowing?: Narrowing;
}

/** The Table layout: a sticky, sortable, resizable header over the virtualised rows. */
export function TableView({ view, narrowing }: Props) {
  const { data } = useViewRows(view.listId, view.id, narrowing);
  if (!data) return null;
  if (Object.keys(data.rows).length === 0) return <EmptyLevel listId={view.listId} viewId={view.id} />;
  return <Table key={view.id} view={view} data={data} />;
}

/** Keyed by the view, so open groups and the selection start fresh on another view. */
function Table({ view, data }: { view: View; data: ViewData }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const { factors, animate } = useGroupMotion();
  const settings = useViewSettings(view);
  const resize = useColumnResize((id, width) => void settings.setWidth(id, width));

  const widthOf = (id: string) => resize.liveWidth(id) ?? columnWidth(id, view.widths);
  // The Table always has its own Status column; a view's Status column would repeat it.
  const columns = data.columns.filter((id) => id !== "status");
  const headerColumns = ["key", "summary", "status", ...columns];
  const minWidth = headerColumns.reduce((sum, id) => sum + (id === "summary" ? SUMMARY_MIN_WIDTH : widthOf(id)), 2 * SIDE_PADDING);

  // The Table's own header names the columns, so an ungrouped view needs no group header. A collapsing group
  // keeps its rows until they have shrunk away.
  const items = useMemo(
    () => flatten(data, (group: Group) => isOpen(group, toggled) || group.key in factors).filter((item) => item.kind === "row" || item.tab),
    [data, toggled, factors],
  );
  const heightOf = (index: number) => TABLE_ROW_HEIGHT * openness(items[index], factors);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: heightOf,
    getItemKey: (index) => itemKey(items[index]),
    scrollMargin: HEADER_HEIGHT,
    paddingEnd: 24,
    overscan: 12,
  });

  // A group in motion changes its rows' heights every frame; the virtualiser caches sizes.
  useLayoutEffect(() => {
    virtualizer.measure();
  }, [factors, virtualizer]);

  const toggle = (group: Group) => {
    const opening = !isOpen(group, toggled);
    setToggled((all) => ({ ...all, [group.key]: opening }));
    animate(group.key, opening);
  };

  return (
    <div ref={scrollRef} className="table-scroll" role="grid" aria-rowcount={items.length}>
      <div className="table-content" style={{ minWidth }}>
        <TableHeader
          columns={headerColumns}
          sort={view.sort}
          widthOf={widthOf}
          onSort={(by) => void settings.setSort(nextSort(view.sort, by))}
          onResizeStart={resize.start}
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
                }}
              >
                {item.kind === "group" ? (
                  <GroupHeader
                    group={item.group}
                    open={item.open}
                    tab
                    active
                    namesColumns={false}
                    headers={[]}
                    onToggle={() => toggle(item.group)}
                  />
                ) : (
                  <TableRow row={item.row} columns={columns} widthOf={widthOf} selected={selected === item.row.id} onSelect={setSelected} />
                )}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
