import { type CSSProperties, useCallback, useMemo, useRef } from "react";
import type { Column, Group, IssueOpening, Narrowing, ViewData } from "../../api";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { useColumnsCatalogue, useViewRows } from "../../queries/views";
import { useStore } from "../../state/store";
import { columnDef } from "./columns";
import { flatten, type Item, itemGroupKey, itemKey } from "./flatten";
import { GroupHeader } from "./GroupHeader";
import { IssueRow } from "./IssueRow";
import { canvasMeasure, columnWidths, fittingColumns, gridTemplate } from "./measure";
import { useElementWidth } from "./useElementWidth";
import { useGroupHighlight } from "./useGroupHighlight";
import { useGroupMotion } from "./useGroupMotion";
import { useIssueOpening } from "../useIssueOpening";
import { useRememberedScroll } from "../useRememberedScroll";
import { useIssueActions } from "./useIssueActions";
import { usePinnedIssues } from "./usePinnedIssues";
import { useRowClicks } from "./useRowClicks";
import { useRowKeys } from "./useRowKeys";
import { useRowSizing } from "./useRowSizing";
import { useRowsPlace } from "./useRowsPlace";

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
  const place = useRowsPlace(listId, viewId, data);
  const { toggled, collapsedParents, selected, setPlace, subtasksOpen } = place;
  const openIssue = useStore((s) => s.openIssue);
  const { factors, moving, animate } = useGroupMotion();
  const highlight = useGroupHighlight();
  const actions = useIssueActions();
  const pinned = usePinnedIssues();
  const width = useElementWidth(scrollRef);
  const { data: catalog = NO_COLUMNS } = useColumnsCatalogue(listId);
  const onScroll = useRememberedScroll(listId, viewId, scrollRef);

  const headers = useMemo(() => data.columns.map((id) => catalog.find((c) => c.id === id)?.name ?? id), [data.columns, catalog]);
  const widths = useMemo(
    () => columnWidths(Object.values(data.rows), data.columns.map((id, i) => ({ id, header: headers[i], cellWidth: columnDef(id).measure })), measure),
    [data.rows, data.columns, headers],
  );
  // Columns that don't fit beside the title (the peek narrows the list) hide from the right.
  const shown = width === null ? widths.length : fittingColumns(widths, width - GRID_INSET);
  const columns = useMemo(() => data.columns.slice(0, shown), [data.columns, shown]);
  const shownHeaders = useMemo(() => headers.slice(0, shown), [headers, shown]);

  // A collapsing group keeps its rows until they have shrunk away.
  const items = useMemo(
    () => flatten(data, (group: Group) => isOpen(group, toggled) || moving.has(group.key), collapsedParents),
    [data, toggled, moving, collapsedParents],
  );
  const rowIds = useMemo(() => items.flatMap((item) => (item.kind === "row" ? [item.row.id] : [])), [items]);
  const { virtualizer, groupStartsMoving, itemProps } = useRowSizing(items, factors, wrapTitles, scrollRef);

  const toggle = (group: Group) => {
    const opening = !isOpen(group, toggled);
    groupStartsMoving(group.key);
    setPlace({ toggled: { ...toggled, [group.key]: opening } });
    animate(group.key, opening);
  };
  const select = (id: string, openPeek: boolean) => place.select(id, openPeek, rowIds);
  const open = (id: string) => openIssue(id, { listId, viewId });
  const scrollToIndex = (index: number) => virtualizer.scrollToIndex(index, { align: "auto" });
  const keys = useRowKeys({ ...place, items, rowIds, data, select, toggle, actions, scrollToIndex, open });

  // Stable, so rows that didn't change skip re-rendering.
  const { onSelect, onOpen } = useRowClicks(keys, opening);
  const onToggleSubtasks = useCallback((id: string) => keys.current.setSubtasksOpen(id, !keys.current.subtasksOpen(id)), [keys]);

  return (
    <div
      ref={scrollRef}
      className="list-scroll"
      role="grid"
      aria-rowcount={items.length}
      style={{ "--list-columns": gridTemplate(widths.slice(0, shown)) } as CSSProperties}
      onScroll={onScroll}
      {...highlight.handlers}
    >
      <div className="list-scroll__space" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((virtual) => {
          const item = items[virtual.index];
          const group = itemGroupKey(item);
          return (
            <div key={itemKey(item)} {...itemProps(virtual, item)} className={itemClass(item)} data-group={group}>
              {item.kind === "group" ? (
                <GroupHeader
                  group={item.group}
                  open={isOpen(item.group, toggled)}
                  tab={item.tab}
                  active={highlight.isActive(group)}
                  namesColumns
                  headers={shownHeaders}
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
                  onSelect={onSelect}
                  onOpen={onOpen}
                  actions={actions}
                  nesting={item.nesting}
                  subtasksOpen={subtasksOpen(item.row.id)}
                  onToggleSubtasks={onToggleSubtasks}
                  childChip={data.subtasks === "hidden" || (item.nesting?.parent === true && !subtasksOpen(item.row.id))}
                  pinned={pinned.has(item.row.id)}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
