import { ChevronLeft } from "lucide-react";
import { api, type BoardColumn, type BoardData, type Narrowing, type View } from "../../api";
import { StatusPill } from "../../components/issue/StatusPill";
import { EmptyLevel } from "../../shell/EmptyLevel";
import { useViewBoard } from "../../queries/views";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { useIssueActions } from "../list/useIssueActions";
import { BoardCard } from "./BoardCard";

interface Props {
  view: View;
  narrowing?: Narrowing;
}

/** The Board layout (artboard 3), read-only: a column per status with its cards; a card opens the Issue view. */
export function BoardView({ view, narrowing }: Props) {
  const { data } = useViewBoard(view.listId, view.id, narrowing);
  if (!data) return null;
  return <Board view={view} data={data} />;
}

function Board({ view, data }: { view: View; data: BoardData }) {
  const openIssue = useStore((s) => s.openIssue);
  const actions = useIssueActions();
  const setCollapsed = (column: BoardColumn, collapsed: boolean) =>
    void attempt(api.viewSetColumnCollapsed(view.id, column.statusId, collapsed), "Could not change the column");
  const open = (id: string) => openIssue(id, { listId: view.listId, viewId: view.id });
  if (data.columns.length === 0) return <EmptyLevel listId={view.listId} viewId={view.id} />;
  return (
    <div className="board" role="list" aria-label="Board">
      {data.columns.map((column) =>
        column.collapsed ? (
          <div key={column.statusId} role="listitem" className="board-strip__item" aria-label={column.name}>
            <button
              type="button"
              className={`board-strip board--${column.style}`}
              aria-label={`Expand ${column.name} (${column.ids.length})`}
              onClick={() => setCollapsed(column, false)}
            >
              <StatusPill style={column.style} name={column.name} variant="vertical" />
              <span className="board-column__count">{column.ids.length}</span>
            </button>
          </div>
        ) : (
          <section key={column.statusId} role="listitem" className={`board-column board--${column.style}`} aria-label={column.name}>
            <header className="board-column__header">
              <StatusPill style={column.style} name={column.name} />
              <span className="board-column__count">{column.ids.length}</span>
              <span className="board-card__spacer" />
              <button type="button" className="board-column__collapse" aria-label="Collapse column" title="Collapse column" onClick={() => setCollapsed(column, true)}>
                <ChevronLeft size={15} strokeWidth={2} aria-hidden="true" />
              </button>
            </header>
            <div className="board-column__cards">
              {column.ids.map((id) => {
                const row = data.rows[id];
                return row ? <BoardCard key={id} row={row} fields={data.cardFields} onOpen={open} actions={actions} /> : null;
              })}
            </div>
          </section>
        ),
      )}
    </div>
  );
}
