import { ChevronLeft } from "lucide-react";
import { useCallback } from "react";
import { api, type BoardColumn, type BoardData, type Narrowing, type View } from "../../api";
import { StatusPill } from "../../components/issue/StatusPill";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { useViewBoard } from "../../queries/views";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { Mark } from "../list/GroupHeader";
import { useIssueActions } from "../list/useIssueActions";
import { BoardColumnCards } from "./BoardColumnCards";
import { useBoardKeys } from "./useBoardKeys";

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
  const setCollapsed = (column: BoardColumn, collapsed: boolean) =>
    void attempt(api.viewSetColumnCollapsed(view.id, column.key, collapsed), "Could not change the column");
  const open = useCallback((id: string) => openIssue(id, { listId: view.listId, viewId: view.id }), [openIssue, view.listId, view.id]);
  const selected = useBoardKeys(data, open, actions);
  if (data.columns.length === 0) return <EmptyLevel listId={view.listId} viewId={view.id} />;
  return (
    <div className="board" role="list" aria-label="Board">
      {data.columns.map((column) =>
        column.collapsed ? (
          <div key={column.key} role="listitem" className="board-strip__item" aria-label={column.name}>
            <button
              type="button"
              className={`board-strip board--${column.style}`}
              aria-label={`Expand ${column.name} (${column.ids.length})`}
              onClick={() => setCollapsed(column, false)}
            >
              <ColumnTitle column={column} vertical />
              <span className="board-column__count">{column.ids.length}</span>
            </button>
          </div>
        ) : (
          <section key={column.key} role="listitem" className={`board-column board--${column.style}`} aria-label={column.name}>
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
              selected={selected !== null && column.ids.includes(selected) ? selected : null}
              onOpen={open}
              actions={actions}
            />
          </section>
        ),
      )}
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
