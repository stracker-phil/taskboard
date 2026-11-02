import { type CSSProperties, useCallback, useMemo, useRef } from "react";
import type { Column, Group, IssueOpening, Narrowing, ViewData } from "../../api";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { useColumnsCatalogue, useViewRows } from "../../queries/views";
import { useStore } from "../../state/store";
import { columnDef } from "./columns";
import { flatten, type Item, itemGroupKey, itemHeight, itemKey } from "./flatten";
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
import { LIST_PADDING_TOP, useRowSizing } from "./useRowSizing";
import { useRowsPlace } from "./useRowsPlace";
import { DropTray, DropZone, zoneOf } from "../../writes/DropZone";
import { foldSpans, type Fold } from "./dragFold";
import { useDragFold } from "./useDragFold";
import { type Drag, useIssueDrag } from "../../writes/useIssueDrag";

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
const MORE_PX = 32;

const isOpen = (group: Group, toggled: Record<string, boolean>) => toggled[group.key] ?? !group.collapsedByDefault;

function itemClass(item: Item): string {
  if (item.kind !== "group") return "list-item";
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
  const { toggled, collapsedParents, selected, picked, setPlace, subtasksOpen } = place;
  const pickedIds = useMemo(() => new Set(picked), [picked]);
  const picking = picked.length > 0;
  const openIssue = useStore((s) => s.openIssue);
  const { factors, moving, animate } = useGroupMotion();
  const highlight = useGroupHighlight();
  const actions = useIssueActions();
  const pinned = usePinnedIssues();
  const width = useElementWidth(scrollRef);
  const { data: catalog = NO_COLUMNS } = useColumnsCatalogue(listId);
  const onScroll = useRememberedScroll(listId, viewId, scrollRef);
  const dropGroups = useMemo(() => data.groups.map((group) => ({ key: group.key, label: group.label })), [data.groups]);
  const { drag, onPointerDown } = useIssueDrag(data.groupBy, dropGroups, actions.openInJira, scrollRef, "y");

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
  const shownItems = useMemo(
    () => flatten(data, (group: Group) => isOpen(group, toggled) || moving.has(group.key), collapsedParents),
    [data, toggled, moving, collapsedParents],
  );
  // While an issue is dragged in a grouped view, every group folds to the same height (`dragFold.ts`).
  const openGroups = useMemo(() => new Set(data.groups.filter((group) => isOpen(group, toggled)).map((group) => group.key)), [data.groups, toggled]);
  const { items, fold } = useDragFold({
    data,
    drag,
    shownItems,
    openGroups,
    collapsedParents,
    scrollRef,
    height: itemHeight,
    paddingTop: LIST_PADDING_TOP,
    morePx: MORE_PX,
  });
  const rowIds = useMemo(() => items.flatMap((item) => (item.kind === "row" ? [item.row.id] : [])), [items]);
  // A closed group's tab opens as its rows fold in.
  const groupOpen = (group: Group) => isOpen(group, toggled) || fold?.open.has(group.key) === true;
  const { virtualizer, groupStartsMoving, itemProps } = useRowSizing(items, factors, wrapTitles, scrollRef, fold);

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
  const { onSelect, onOpen, onPick, onToggleBox } = useRowClicks(keys, opening);
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
            <div
              key={itemKey(item)}
              {...itemProps(virtual, item)}
              className={itemClass(item)}
              data-group={group}
              onPointerDown={
                item.kind === "row"
                  ? (e) => onPointerDown(e, { id: item.row.id, key: item.row.key, summary: item.row.summary, fromKey: group }, e.currentTarget)
                  : undefined
              }
            >
              {item.kind === "more" ? (
                <MorePiece item={item} />
              ) : item.kind === "group" ? (
                <GroupHeader
                  group={item.group}
                  open={groupOpen(item.group)}
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
                  position={fold?.positions.get(virtual.index) ?? item.position}
                  wrap={wrapTitles}
                  inlineLabels={inlineLabels}
                  selected={picking ? pickedIds.has(item.row.id) : selected === item.row.id}
                  picked={pickedIds.has(item.row.id)}
                  picking={picking}
                  onSelect={onSelect}
                  onToggleBox={onToggleBox}
                  onPick={onPick}
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
        {drag && fold && <GroupZones drag={drag} items={items} fold={fold} />}
      </div>
      <DropTray drag={drag} groups={dropGroups} host={scrollRef} />
    </div>
  );
}

/** The "+N more" piece that closes a folded group's card. */
function MorePiece({ item }: { item: Extract<Item, { kind: "more" }> }) {
  return <div className={item.tab ? "list-more list-more--tabbed" : "list-more"}>{item.hidden > 0 ? `+${item.hidden} more` : ""}</div>;
}

/** Each group's drop zone, laid exactly over its card below the header. */
function GroupZones({ drag, items, fold }: { drag: Drag; items: Item[]; fold: Fold }) {
  return (
    <>
      {[...foldSpans(items, fold.heights, LIST_PADDING_TOP)].map(([key, span]) => {
        const zone = zoneOf(drag, key);
        return zone ? <DropZone key={key} zone={zone} className={span.tab ? "drop-zone--tabbed" : ""} style={{ top: span.top, height: span.height }} /> : null;
      })}
    </>
  );
}
