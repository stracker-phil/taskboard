import { useVirtualizer } from "@tanstack/react-virtual";
import { useMemo, useRef, useState } from "react";
import type { Group, Narrowing, View, ViewData } from "../../api";
import { useViewRows } from "../../queries/views";
import { flatten, GROUP_HEIGHT, type Item } from "../list/flatten";
import { GroupHeader } from "../list/GroupHeader";
import { nextSort } from "./sortable";
import { TableHeader } from "./TableHeader";
import { TableRow } from "./TableRow";
import { useColumnResize } from "./useColumnResize";
import { useViewSettings } from "./useViewSettings";
import { columnWidth, SUMMARY_MIN_WIDTH } from "./widths";

const TABLE_ROW_HEIGHT = 36;
const HEADER_HEIGHT = 34;
const SIDE_PADDING = 24;

const TABLE_GROUP_GAP = 14;

const itemHeight = (item: Item) => (item.kind === "row" ? TABLE_ROW_HEIGHT : item.first ? GROUP_HEIGHT : GROUP_HEIGHT + TABLE_GROUP_GAP);

interface Props {
  view: View;
  narrowing?: Narrowing;
}

/** The Table layout: a sticky, sortable, resizable header over the virtualised rows. */
export function TableView({ view, narrowing }: Props) {
  const { data } = useViewRows(view.listId, view.id, narrowing);
  if (!data) return null;
  return <Table key={view.id} view={view} data={data} />;
}

/** Keyed by the view, so open groups and the selection start fresh on another view. */
function Table({ view, data }: { view: View; data: ViewData }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const [selected, setSelected] = useState<string | null>(null);
  const settings = useViewSettings(view);
  const resize = useColumnResize((id, width) => void settings.setWidth(id, width));

  const widthOf = (id: string) => resize.liveWidth(id) ?? columnWidth(id, view.widths);
  // The Table always has its own Status column; a view's Status column would repeat it.
  const columns = data.columns.filter((id) => id !== "status");
  const headerColumns = ["key", "summary", "status", ...columns];
  const minWidth = headerColumns.reduce((sum, id) => sum + (id === "summary" ? SUMMARY_MIN_WIDTH : widthOf(id)), 2 * SIDE_PADDING);

  // The Table's own header names the columns, so an ungrouped view needs no group header.
  const items = useMemo(
    () => flatten(data, (group: Group) => toggled[group.key] ?? !group.collapsedByDefault).filter((item) => item.kind === "row" || item.tab),
    [data, toggled],
  );
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => itemHeight(items[index]),
    scrollMargin: HEADER_HEIGHT,
    paddingEnd: 24,
    overscan: 12,
  });

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
            return (
              <div
                key={item.kind === "row" ? `row:${item.row.id}` : `group:${item.group.key}`}
                className={item.kind === "group" && !item.first ? "table-item table-item--gap" : "table-item"}
                style={{ height: virtual.size, transform: `translateY(${virtual.start - HEADER_HEIGHT}px)` }}
              >
                {item.kind === "group" ? (
                  <GroupHeader
                    group={item.group}
                    open={item.open}
                    tab
                    active
                    namesColumns={false}
                    headers={[]}
                    onToggle={() => setToggled((all) => ({ ...all, [item.group.key]: !item.open }))}
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
