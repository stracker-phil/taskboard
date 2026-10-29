import { useQueryClient } from "@tanstack/react-query";
import { type CSSProperties, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Column, Group, IssueOpening, Narrowing, ViewData, ViewPlace } from "../../api";
import { onIssueKeys } from "../../commands/issueKeys";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { prefetchIssue, refreshIssueExtras } from "../../queries/issues";
import { useColumnsCatalogue, useViewRows } from "../../queries/views";
import { narrowingKey } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";
import { columnDef } from "./columns";
import { flatten, isBelow, type Item, itemGroupKey, itemKey } from "./flatten";
import { GroupHeader } from "./GroupHeader";
import { IssueRow } from "./IssueRow";
import { canvasMeasure, columnWidths, fittingColumns, gridTemplate } from "./measure";
import { useElementWidth } from "./useElementWidth";
import { useGroupHighlight } from "./useGroupHighlight";
import { useGroupMotion } from "./useGroupMotion";
import { useIssueOpening } from "../useIssueOpening";
import { useIssueActions } from "./useIssueActions";
import { useRowSizing } from "./useRowSizing";

interface Props {
  listId: number;
  viewId: number;
  narrowing?: Narrowing;
  /** Whether long titles wrap instead of truncating. */
  wrapTitles?: boolean;
  /** What a click opens; none follows Settings → General. */
  openWith?: IssueOpening | null;
  /** Labels as tags after the title, unless the view has a Labels column. */
  inlineLabels?: boolean;
}

/** The List layout: each group a card under its tab, virtualized. */
export function ListView({ listId, viewId, narrowing, wrapTitles = true, openWith = null, inlineLabels = true }: Props) {
  const { data } = useViewRows(listId, viewId, narrowing);
  const opening = useIssueOpening(openWith);
  if (!data) return null;
  if (Object.keys(data.rows).length === 0) return <EmptyLevel listId={listId} viewId={viewId} />;
  return <Rows key={`${listId}:${viewId}`} listId={listId} viewId={viewId} data={data} wrapTitles={wrapTitles} opening={opening} inlineLabels={inlineLabels && !data.columns.includes("labels")} />;
}

const measure = canvasMeasure();
const NO_COLUMNS: Column[] = [];
const NO_TOGGLED: Record<string, boolean> = {};
const NO_PARENTS: string[] = [];
/** The list's side padding plus a row's: what the grid can't use. */
const GRID_INSET = 52 + 24 + 32;

const isOpen = (group: Group, toggled: Record<string, boolean>) => toggled[group.key] ?? !group.collapsedByDefault;

function itemClass(item: Item): string {
  if (item.kind === "row") return "list-item";
  return ["list-item", !item.first && "list-item--gap", !item.tab && "list-item--tabless"].filter(Boolean).join(" ");
}

/** Keyed by the view; its open groups, scroll, selection and peek come from the view's remembered place. */
interface RowsProps {
  listId: number;
  viewId: number;
  data: ViewData;
  wrapTitles: boolean;
  opening: IssueOpening;
  inlineLabels: boolean;
}

function Rows({ listId, viewId, data, wrapTitles, opening, inlineLabels }: RowsProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const placeKey = narrowingKey(listId, viewId);
  const loaded = useStore((s) => placeKey in s.places);
  const toggled = useStore((s) => s.places[placeKey]?.toggled) ?? NO_TOGGLED;
  const collapsedParents = useStore((s) => s.places[placeKey]?.collapsedParents) ?? NO_PARENTS;
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
  const { data: catalog = NO_COLUMNS } = useColumnsCatalogue(listId);

  useEffect(() => {
    void loadPlace(listId, viewId);
  }, [loadPlace, listId, viewId]);

  const headers = useMemo(() => data.columns.map((id) => catalog.find((c) => c.id === id)?.name ?? id), [data.columns, catalog]);
  const widths = useMemo(
    () => columnWidths(Object.values(data.rows), data.columns.map((id, i) => ({ id, header: headers[i], cellWidth: columnDef(id).measure })), measure),
    [data.rows, data.columns, headers],
  );
  // Columns that don't fit beside the title (the peek narrows the list) hide from the right.
  const shown = width === null ? widths.length : fittingColumns(widths, width - GRID_INSET);
  const columns = data.columns.slice(0, shown);

  // A collapsing group keeps its rows until they have shrunk away.
  const items = useMemo(
    () => flatten(data, (group: Group) => isOpen(group, toggled) || group.key in factors, collapsedParents),
    [data, toggled, factors, collapsedParents],
  );
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

  // Collapsing a row moves the selection from its hidden subtree onto it.
  const setSubtasksOpen = (id: string, open: boolean) => {
    setPlace({ collapsedParents: open ? collapsedParents.filter((p) => p !== id) : [...collapsedParents, id] });
    if (!open && selected && isBelow(data.rows, selected, id)) setPlace({ selectedId: id });
  };
  const subtasksOpen = (id: string) => !collapsedParents.includes(id);

  const rowIds = items.flatMap((item) => (item.kind === "row" ? [item.row.id] : []));
  const select = (id: string, openPeek: boolean) => {
    setPlace({ selectedId: id, peekOpen: openPeek });
    refreshIssueExtras(id);
    const at = rowIds.indexOf(id);
    for (const neighbor of [rowIds[at - 1], rowIds[at + 1]]) if (neighbor) prefetchIssue(client, neighbor);
  };

  // The keys reach this render's state through a ref the effect below keeps current.
  const open = (id: string) => openIssue(id, { listId, viewId });
  const current = { items, rowIds, selected, peekOpen, data, select, toggle, setSubtasksOpen, subtasksOpen, actions, virtualizer, setPlace, open };
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
        const { items, selected, toggle, select, peekOpen, setSubtasksOpen, subtasksOpen } = keyState.current;
        const row = items.find((item) => item.kind === "row" && item.row.id === selected);
        // In Hierarchy ←/→ on a row with children closes or opens its subtree, and ← on a nested row selects its
        // parent.
        if (row?.kind === "row" && row.nesting?.parent && subtasksOpen(row.row.id) !== open) {
          setSubtasksOpen(row.row.id, open);
          return;
        }
        if (row?.kind === "row" && row.nesting && row.nesting.depth > 0 && !open && row.row.parentId) {
          select(row.row.parentId, peekOpen);
          return;
        }
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
      togglePin: () => {
        const { selected, actions } = keyState.current;
        if (selected) actions.togglePin(selected);
      },
      toggleRead: () => {
        const { selected, actions, data } = keyState.current;
        if (!selected) return;
        if (data.rows[selected]?.changed) actions.markSeen(selected);
        else actions.markUnread(selected);
      },
      togglePeek: () => {
        const { selected, peekOpen, setPlace } = keyState.current;
        if (selected) setPlace({ peekOpen: !peekOpen });
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
              key={itemKey(item)}
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
                  inlineLabels={inlineLabels}
                  selected={selected === item.row.id}
                  onSelect={(id) => {
                    select(id, opening === "preview");
                    if (opening === "full") open(id);
                  }}
                  onOpen={open}
                  actions={actions}
                  nesting={item.nesting}
                  subtasksOpen={subtasksOpen(item.row.id)}
                  onToggleSubtasks={(id) => setSubtasksOpen(id, !subtasksOpen(id))}
                  childChip={data.subtasks === "hidden" || (item.nesting?.parent === true && !subtasksOpen(item.row.id))}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
