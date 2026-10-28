import { useQueryClient } from "@tanstack/react-query";
import { type CSSProperties, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Column, Group, Narrowing, ViewData, ViewPlace } from "../../api";
import { onIssueKeys } from "../../commands/issueKeys";
import { prefetchIssue } from "../../queries/issues";
import { useColumnsCatalogue, useViewRows } from "../../queries/views";
import { narrowingKey } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";
import { columnDef } from "./columns";
import { flatten, type Item, itemGroupKey } from "./flatten";
import { GroupHeader } from "./GroupHeader";
import { IssueRow } from "./IssueRow";
import { canvasMeasure, columnWidths, fittingColumns, gridTemplate } from "./measure";
import { useElementWidth } from "./useElementWidth";
import { useGroupHighlight } from "./useGroupHighlight";
import { useGroupMotion } from "./useGroupMotion";
import { useIssueActions } from "./useIssueActions";
import { useRowSizing } from "./useRowSizing";

interface Props {
  listId: number;
  viewId: number;
  narrowing?: Narrowing;
  /** Whether long titles wrap instead of truncating. */
  wrapTitles?: boolean;
}

/** The List layout: each group a card under its tab, virtualised. */
export function ListView({ listId, viewId, narrowing, wrapTitles = true }: Props) {
  const { data } = useViewRows(listId, viewId, narrowing);
  if (!data) return null;
  return <Rows key={`${listId}:${viewId}`} listId={listId} viewId={viewId} data={data} wrapTitles={wrapTitles} />;
}

const measure = canvasMeasure();
const NO_COLUMNS: Column[] = [];
const NO_TOGGLED: Record<string, boolean> = {};
/** The list's side padding plus a row's: what the grid can't use. */
const GRID_INSET = 52 + 24 + 32;

const isOpen = (group: Group, toggled: Record<string, boolean>) => toggled[group.key] ?? !group.collapsedByDefault;

function itemClass(item: Item): string {
  if (item.kind === "row") return "list-item";
  return ["list-item", !item.first && "list-item--gap", !item.tab && "list-item--tabless"].filter(Boolean).join(" ");
}

/** Keyed by the view; its open groups, scroll, selection and peek come from the view's remembered place. */
function Rows({ listId, viewId, data, wrapTitles }: { listId: number; viewId: number; data: ViewData; wrapTitles: boolean }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const placeKey = narrowingKey(listId, viewId);
  const loaded = useStore((s) => placeKey in s.places);
  const toggled = useStore((s) => s.places[placeKey]?.toggled) ?? NO_TOGGLED;
  const selected = useStore((s) => s.places[placeKey]?.selectedId ?? null);
  const peekOpen = useStore((s) => s.places[placeKey]?.peekOpen ?? false);
  const loadPlace = useStore((s) => s.loadPlace);
  const updatePlace = useStore((s) => s.updatePlace);
  const openIssue = useStore((s) => s.openIssue);
  const setPlace = (patch: Partial<ViewPlace>) => updatePlace(listId, viewId, patch);
  const { factors, animate } = useGroupMotion();
  const highlight = useGroupHighlight();
  const actions = useIssueActions();
  const client = useQueryClient();
  const width = useElementWidth(scrollRef);
  const { data: catalogue = NO_COLUMNS } = useColumnsCatalogue();

  useEffect(() => {
    void loadPlace(listId, viewId);
  }, [loadPlace, listId, viewId]);

  const headers = useMemo(() => data.columns.map((id) => catalogue.find((c) => c.id === id)?.name ?? id), [data.columns, catalogue]);
  const widths = useMemo(
    () => columnWidths(Object.values(data.rows), data.columns.map((id, i) => ({ id, header: headers[i], cellWidth: columnDef(id).measure })), measure),
    [data.rows, data.columns, headers],
  );
  // Columns that don't fit beside the title (the peek narrows the list) hide from the right.
  const shown = width === null ? widths.length : fittingColumns(widths, width - GRID_INSET);
  const columns = data.columns.slice(0, shown);

  // A collapsing group keeps its rows until they have shrunk away.
  const items = useMemo(() => flatten(data, (group: Group) => isOpen(group, toggled) || group.key in factors), [data, toggled, factors]);
  const { virtualizer, groupStartsMoving, itemProps } = useRowSizing(items, factors, wrapTitles, scrollRef);

  // Back to where the owner left this view, once its place has loaded.
  const [restored, setRestored] = useState(false);
  useLayoutEffect(() => {
    if (!loaded || restored || !scrollRef.current) return;
    scrollRef.current.scrollTop = useStore.getState().places[placeKey]?.scrollTop ?? 0;
    setRestored(true);
  }, [loaded, restored, placeKey]);

  const toggle = (group: Group) => {
    const opening = !isOpen(group, toggled);
    groupStartsMoving(group.key);
    setPlace({ toggled: { ...toggled, [group.key]: opening } });
    animate(group.key, opening);
  };

  const rowIds = items.flatMap((item) => (item.kind === "row" ? [item.row.id] : []));
  const select = (id: string, openPeek: boolean) => {
    setPlace({ selectedId: id, peekOpen: openPeek });
    const at = rowIds.indexOf(id);
    for (const neighbour of [rowIds[at - 1], rowIds[at + 1]]) if (neighbour) prefetchIssue(client, neighbour);
  };

  // The keys reach this render's state through a ref the effect below keeps current.
  const open = (id: string) => openIssue(id, { listId, viewId });
  const current = { items, rowIds, selected, peekOpen, data, select, toggle, actions, virtualizer, setPlace, open };
  const keyState = useRef(current);
  useEffect(() => {
    keyState.current = current;
  });
  useEffect(() => {
    const withSelected = (run: (key: string) => void) => {
      const { data, selected } = keyState.current;
      const row = selected ? data.rows[selected] : undefined;
      if (row) run(row.key);
    };
    return onIssueKeys({
      move: (step) => {
        const { rowIds, selected, peekOpen, items, virtualizer, select } = keyState.current;
        if (rowIds.length === 0) return;
        const at = selected === null ? -1 : rowIds.indexOf(selected);
        const next = rowIds[Math.min(Math.max(at + step, 0), rowIds.length - 1)];
        select(next, peekOpen);
        virtualizer.scrollToIndex(
          items.findIndex((item) => item.kind === "row" && item.row.id === next),
          { align: "auto" },
        );
      },
      setGroupOpen: (open) => {
        const { items, selected, toggle } = keyState.current;
        const row = items.find((item) => item.kind === "row" && item.row.id === selected);
        const header = row && items.find((item) => item.kind === "group" && item.group.key === itemGroupKey(row));
        if (header?.kind === "group" && header.open !== open) toggle(header.group);
      },
      closePeek: () => keyState.current.setPlace({ peekOpen: false }),
      copyKey: () => withSelected((key) => keyState.current.actions.copyKey(key)),
      copyUrl: () => withSelected((key) => keyState.current.actions.copyUrl(key)),
      openInJira: () => withSelected((key) => keyState.current.actions.openInJira?.(key)),
      open: () => {
        const { selected, open } = keyState.current;
        if (selected) open(selected);
      },
      openEpic: () => {
        const { data, selected, open } = keyState.current;
        const epic = selected ? data.rows[selected]?.epic : undefined;
        if (epic) open(epic.id);
      },
      hasSelection: () => keyState.current.selected !== null,
      peekOpen: () => keyState.current.peekOpen,
    });
  }, []);

  return (
    <div
      ref={scrollRef}
      className="list-scroll"
      role="grid"
      aria-rowcount={items.length}
      style={{ "--list-columns": gridTemplate(widths.slice(0, shown)) } as CSSProperties}
      onScroll={(e) => setPlace({ scrollTop: e.currentTarget.scrollTop })}
      {...highlight.handlers}
    >
      <div className="list-scroll__space" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtual) => {
          const item = items[virtual.index];
          const group = itemGroupKey(item);
          return (
            <div
              key={item.kind === "row" ? `row:${item.row.id}` : `group:${item.group.key}`}
              {...itemProps(virtual, item)}
              className={itemClass(item)}
              data-group={group}
            >
              {item.kind === "group" ? (
                <GroupHeader
                  group={item.group}
                  open={isOpen(item.group, toggled)}
                  tab={item.tab}
                  active={highlight.isActive(group)}
                  namesColumns
                  headers={headers.slice(0, shown)}
                  onToggle={() => toggle(item.group)}
                />
              ) : (
                <IssueRow
                  row={item.row}
                  columns={columns}
                  position={item.position}
                  wrap={wrapTitles}
                  selected={selected === item.row.id}
                  onSelect={(id) => select(id, true)}
                  actions={actions}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
