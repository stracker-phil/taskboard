import { useVirtualizer } from "@tanstack/react-virtual";
import { type CSSProperties, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Group, Narrowing, ViewData } from "../../api";
import { useColumnsCatalogue, useViewRows } from "../../queries/views";
import { columnDef } from "./columns";
import { flatten, type Item, itemGroupKey, itemHeight } from "./flatten";
import { GroupHeader } from "./GroupHeader";
import { IssueRow } from "./IssueRow";
import { canvasMeasure, listColumns } from "./measure";
import { useGroupMotion } from "./useGroupMotion";

interface Props {
  listId: number;
  viewId: number;
  narrowing?: Narrowing;
}

/** The List layout (artboard 1): each group a card under its tab, virtualised. */
export function ListView({ listId, viewId, narrowing }: Props) {
  const { data } = useViewRows(listId, viewId, narrowing);
  if (!data) return null;
  return <Rows key={`${listId}:${viewId}`} data={data} />;
}

const measure = canvasMeasure();

/** A row of a group in motion fades with its height. */
function motionStyle(item: Item, factors: Record<string, number>): CSSProperties | undefined {
  if (item.kind !== "row" || !(item.groupKey in factors)) return undefined;
  return { opacity: factors[item.groupKey], overflow: "hidden" };
}

function itemClass(item: Item): string {
  if (item.kind === "row") return "list-item";
  return ["list-item", !item.first && "list-item--gap", !item.tab && "list-item--tabless"].filter(Boolean).join(" ");
}

/** Keyed by the view, so open groups and the selection start fresh on another view. */
function Rows({ data }: { data: ViewData }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  // Groups the owner opened or closed; the others follow `collapsedByDefault`.
  const [toggled, setToggled] = useState<Record<string, boolean>>({});
  const { factors, animate } = useGroupMotion();
  const openOf = (group: Group) => toggled[group.key] ?? !group.collapsedByDefault;
  const [selected, setSelected] = useState<string | null>(null);
  const [hovered, setHovered] = useState<string | null>(null);
  const [focused, setFocused] = useState<string | null>(null);
  const { data: catalogue = [] } = useColumnsCatalogue();

  const headers = useMemo(
    () => data.columns.map((id) => catalogue.find((c) => c.id === id)?.name ?? id),
    [data.columns, catalogue],
  );
  const template = useMemo(() => {
    const rows = Object.values(data.rows);
    const columns = data.columns.map((id, index) => ({ id, header: headers[index], cellWidth: columnDef(id).measure }));
    return listColumns(rows, columns, measure);
  }, [data.rows, data.columns, headers]);

  // A collapsing group keeps its rows until they have shrunk away.
  const items = useMemo(
    () => flatten(data, (group: Group) => (toggled[group.key] ?? !group.collapsedByDefault) || group.key in factors),
    [data, toggled, factors],
  );
  const heightOf = (index: number) => {
    const item = items[index];
    return item.kind === "row" ? itemHeight(item) * (factors[item.groupKey] ?? 1) : itemHeight(item);
  };
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: heightOf,
    paddingStart: 22,
    paddingEnd: 40,
    overscan: 12,
  });

  useLayoutEffect(() => {
    virtualizer.measure();
  }, [factors, virtualizer]);

  const toggle = (group: Group) => {
    const opening = !openOf(group);
    setToggled((all) => ({ ...all, [group.key]: opening }));
    animate(group.key, opening);
  };

  /** The group of the item an event happened in, from the `data-group` every item carries. */
  const groupAt = (target: EventTarget | null) =>
    target instanceof Element ? (target.closest<HTMLElement>("[data-group]")?.dataset.group ?? null) : null;

  return (
    <div
      ref={scrollRef}
      className="list-scroll"
      role="grid"
      aria-rowcount={items.length}
      style={{ "--list-columns": template } as CSSProperties}
      onPointerOver={(e) => setHovered(groupAt(e.target))}
      onPointerLeave={() => setHovered(null)}
      onFocus={(e) => setFocused(groupAt(e.target))}
      onBlur={() => setFocused(null)}
    >
      <div className="list-scroll__space" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtual) => {
          const item = items[virtual.index];
          const group = itemGroupKey(item);
          return (
            <div
              key={item.kind === "row" ? `row:${item.row.id}` : `group:${item.group.key}`}
              className={itemClass(item)}
              data-group={group}
              style={{ height: virtual.size, transform: `translateY(${virtual.start}px)`, ...motionStyle(item, factors) }}
            >
              {item.kind === "group" ? (
                <GroupHeader
                  group={item.group}
                  open={openOf(item.group)}
                  tab={item.tab}
                  active={hovered === group || focused === group}
                  headers={headers}
                  onToggle={() => toggle(item.group)}
                />
              ) : (
                <IssueRow row={item.row} columns={data.columns} position={item.position} selected={selected === item.row.id} onSelect={setSelected} />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
