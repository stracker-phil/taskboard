import { useGlide, useHeightGlide } from "../../lib/motion";
import { ChevronLeft } from "lucide-react";
import { type PointerEvent, type RefObject, useCallback, useEffect, useRef } from "react";
import { api, type BoardColumn, type BoardData, type Narrowing, type View } from "../../api";
import { StatusPill } from "../../components/issue/StatusPill";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { useViewBoard } from "../../queries/views";
import { attempt } from "../../state/requests";
import { narrowingKey } from "../../state/slices/narrowing";
import { useStore } from "../../state/store";
import { Mark } from "../list/GroupHeader";
import { useIssueActions } from "../list/useIssueActions";
import { usePinnedIssues } from "../list/usePinnedIssues";
import { BoardColumnCards } from "./BoardColumnCards";
import { useBoardKeys } from "./useBoardKeys";
import { colorClass, DropTray, DropZone, zoneOf } from "../../writes/DropZone";
import { type Drag, useIssueDrag } from "../../writes/useIssueDrag";

interface Props {
  view: View;
  narrowing?: Narrowing;
}

/** The Board layout (artboard 3), read-only: a column per group of the view (per status by default) with its
 *  cards; a card opens the Issue view. */
export function BoardView({ view, narrowing }: Props) {
  const { data } = useViewBoard(view.listId, view.id, narrowing);
  if (!data) return null;
  return <Board view={view} data={data} />;
}

function Board({ view, data }: { view: View; data: BoardData }) {
  const openIssue = useStore((s) => s.openIssue);
  const actions = useIssueActions();
  const pinned = usePinnedIssues();
  const setCollapsed = (column: BoardColumn, collapsed: boolean) =>
    void attempt(api.viewSetColumnCollapsed(view.id, column.key, collapsed), "Could not change the column");
  const { listId, id: viewId } = view;
  const placeKey = narrowingKey(listId, viewId);
  const selected = useStore((s) => s.places[placeKey]?.selectedId ?? null);
  const loadPlace = useStore((s) => s.loadPlace);
  const updatePlace = useStore((s) => s.updatePlace);
  useEffect(() => {
    void loadPlace(listId, viewId);
  }, [loadPlace, listId, viewId]);
  const select = useCallback((id: string) => updatePlace(listId, viewId, { selectedId: id }), [updatePlace, listId, viewId]);
  const open = useCallback((id: string) => openIssue(id, { listId, viewId }), [openIssue, listId, viewId]);
  // A clicked card becomes the current one, so the Board comes back to it.
  const openCard = useCallback(
    (id: string) => {
      select(id);
      open(id);
    },
    [select, open],
  );
  useBoardKeys(data, selected, select, open, actions);
  // A Board that groups by nothing shows status columns.
  const columnGroups = data.columns.map((column) => ({ key: column.key, label: column.name }));
  const boardRef = useRef<HTMLDivElement>(null);
  const { drag, onPointerDown } = useIssueDrag(view.groupBy === "none" ? "status" : view.groupBy, columnGroups, actions.openInJira, boardRef, "x");
  const dragCard = (e: PointerEvent<HTMLElement>, column: BoardColumn) => {
    const card = (e.target as HTMLElement).closest<HTMLElement>("[data-issue-id]");
    const row = card?.dataset.issueId ? data.rows[card.dataset.issueId] : undefined;
    if (card && row) onPointerDown(e, { id: row.id, key: row.key, summary: row.summary, fromKey: column.key }, card);
  };
  useCurrentColumnInView(boardRef, data, selected);
  // A column that collapses or opens moves the ones after it smoothly.
  useGlide(boardRef, view.id);
  // While an issue is dragged every column reaches the board's height, and glides there and back.
  useHeightGlide(boardRef, drag !== null);
  const inlineLabels = view.inlineLabels && !data.cardFields.includes("labels");
  if (data.columns.length === 0) return <EmptyLevel listId={view.listId} viewId={view.id} />;
  return (
    <div ref={boardRef} className={drag ? "board board--dragging" : "board"} role="list" aria-label="Board">
      {data.columns.map((column) =>
        column.collapsed ? (
          <div key={column.key} role="listitem" className={`board-strip__item ${zoneColor(drag, column.key)}`} aria-label={column.name}>
            <button
              type="button"
              className={`board-strip board--${column.style}`}
              aria-label={`Expand ${column.name} (${column.ids.length})`}
              onClick={() => setCollapsed(column, false)}
            >
              <ColumnTitle column={column} vertical />
              <span className="board-column__count board-column__count--vertical">{column.ids.length}</span>
            </button>
            <ColumnZone drag={drag} columnKey={column.key} strip />
          </div>
        ) : (
          <section
            key={column.key}
            data-column={column.key}
            role="listitem"
            className={`board-column board--${column.style} ${zoneColor(drag, column.key)}`}
            aria-label={column.name}
            onPointerDown={(e) => dragCard(e, column)}
          >
            <header className="board-column__header">
              <ColumnTitle column={column} />
              <span className="board-column__count">{column.ids.length}</span>
              <span className="board-card__spacer" />
              <button type="button" className="board-column__collapse" aria-label="Collapse column" title="Collapse column" onClick={() => setCollapsed(column, true)}>
                <ChevronLeft size={15} strokeWidth={2} aria-hidden="true" />
              </button>
            </header>
            <BoardColumnCards
              ids={column.ids}
              rows={data.rows}
              fields={data.cardFields}
              inlineLabels={inlineLabels}
              selected={selected !== null && column.ids.includes(selected) ? selected : null}
              onOpen={openCard}
              actions={actions}
              pinned={pinned}
            />
            <ColumnZone drag={drag} columnKey={column.key} />
          </section>
        ),
      )}
      <DropTray drag={drag} groups={columnGroups} host={boardRef} />
    </div>
  );
}

/** A status column shows its status pill; any other grouping a neutral pill with the List's group mark. */
function ColumnTitle({ column, vertical = false }: { column: BoardColumn; vertical?: boolean }) {
  if (column.mark?.kind === "status") return <StatusPill style={column.style} name={column.name} variant={vertical ? "vertical" : "default"} />;
  const className = ["status-pill", "status-pill--todo", "board-column__group", vertical && "status-pill--vertical"].filter(Boolean).join(" ");
  return (
    <span className={className}>
      <Mark mark={column.mark} />
      {column.name}
    </span>
  );
}

/** Once the Board comes back to its current card, scrolls that card's column into view; the column scrolls
 *  to the card itself. */
function useCurrentColumnInView(boardRef: RefObject<HTMLDivElement | null>, data: BoardData, selected: string | null) {
  const done = useRef(false);
  useEffect(() => {
    if (done.current || !selected) return;
    const column = data.columns.find((c) => !c.collapsed && c.ids.includes(selected));
    if (!column) return;
    done.current = true;
    const element = [...(boardRef.current?.querySelectorAll<HTMLElement>("[data-column]") ?? [])].find((e) => e.dataset.column === column.key);
    element?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
  }, [boardRef, data, selected]);
}

/** The group color a column glows in while the pointer is over its zone. */
function zoneColor(drag: Drag | null, columnKey: string): string {
  const target = zoneOf(drag, columnKey)?.target;
  return target ? colorClass(target.mark) : "";
}

/** A column's drop zone: the whole column (they all reach the board's height while dragging), or a collapsed
 *  column's strip. */
function ColumnZone({ drag, columnKey, strip = false }: { drag: Drag | null; columnKey: string; strip?: boolean }) {
  const zone = zoneOf(drag, columnKey);
  return zone ? <DropZone zone={zone} className={strip ? "drop-zone--strip" : "drop-zone--column"} /> : null;
}
