import { useVirtualizer } from "@tanstack/react-virtual";
import { useMemo, useRef, useState } from "react";
import type { Group, Narrowing, ViewData } from "../../api";
import { useViewRows } from "../../queries/views";
import { ColumnHeader } from "./ColumnHeader";
import { flatten, itemHeight } from "./flatten";
import { GroupHeader } from "./GroupHeader";
import { IssueRow } from "./IssueRow";

interface Props {
  listId: number;
  viewId: number;
  narrowing?: Narrowing;
}

/** The List layout (artboard 1): the column header over the grouped, virtualised rows. */
export function ListView({ listId, viewId, narrowing }: Props) {
  const { data } = useViewRows(listId, viewId, narrowing);
  if (!data) return null;
  return (
    <div className="list-view">
      <ColumnHeader columns={data.columns} />
      <Rows key={`${listId}:${viewId}`} data={data} />
    </div>
  );
}

/** Keyed by the view, so open groups and the selection start fresh on another view. */
function Rows({ data }: { data: ViewData }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  // Groups the owner opened or closed; the others follow `collapsedByDefault`.
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const [selected, setSelected] = useState<string | null>(null);

  const items = useMemo(() => flatten(data, (group: Group) => toggled[group.key] ?? !group.collapsedByDefault), [data, toggled]);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: (index) => itemHeight(items[index]),
    paddingStart: 8,
    paddingEnd: 24,
    overscan: 12,
  });

  return (
    <div ref={scrollRef} className="list-scroll" role="grid" aria-rowcount={items.length}>
      <div className="list-scroll__space" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtual) => {
          const item = items[virtual.index];
          return (
            <div
              key={item.kind === "row" ? `row:${item.row.id}` : `group:${item.group.key}`}
              className={item.kind === "group" && !item.first ? "list-item list-item--gap" : "list-item"}
              style={{ height: virtual.size, transform: `translateY(${virtual.start}px)` }}
            >
              {item.kind === "group" ? (
                <GroupHeader group={item.group} open={item.open} onToggle={() => setToggled((all) => ({ ...all, [item.group.key]: !item.open }))} />
              ) : (
                <IssueRow row={item.row} columns={data.columns} selected={selected === item.row.id} onSelect={setSelected} />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
