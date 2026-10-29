import { memo } from "react";
import type { Row } from "../../api";
import { AvatarStack } from "../../components/issue/Avatar";
import { CommentCount } from "../../components/issue/CommentCount";
import { EpicChip } from "../../components/issue/EpicChip";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { RowActions, type RowActionHandlers } from "../list/RowActions";

interface Props {
  row: Row;
  /** The view's card fields: which of epic, assignee, release and priority show. */
  fields: string[];
  /** The keyboard's current card. */
  selected?: boolean;
  onOpen: (id: string) => void;
  actions: RowActionHandlers;
}

/** A Board card: epic and key, the summary (always wrapped, never cut), then people, release, comments and
 *  priority; changed cards are tinted. */
export const BoardCard = memo(function BoardCard({ row, fields, selected = false, onOpen, actions }: Props) {
  const shows = (field: string) => fields.includes(field);
  return (
    <article
      className={["board-card", row.changed && "board-card--changed", selected && "board-card--selected"].filter(Boolean).join(" ")}
      aria-current={selected || undefined}
      tabIndex={0}
      aria-label={`${row.key} ${row.summary}`}
      onClick={() => onOpen(row.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" && e.target === e.currentTarget) onOpen(row.id);
      }}
    >
      <div className="board-card__top">
        {shows("epic") && row.epic && <EpicChip epic={row.epic} />}
        <span className="board-card__spacer" />
        {row.changed && <span className="board-card__dot" title="Changed since yesterday" />}
        <span className="board-card__key">{row.key}</span>
      </div>
      <p className="board-card__summary">{row.summary}</p>
      <div className="board-card__footer">
        {shows("assignee") && <AvatarStack people={row.assignees} />}
        <span className="board-card__spacer" />
        {shows("release") && row.release && <span className="board-card__release">{row.release}</span>}
        <CommentCount count={row.commentCount} />
        {shows("priority") && row.priority && (
          <span className="board-card__priority" role="img" aria-label={`${row.priority.name} priority`} title={`${row.priority.name} priority`}>
            <PriorityFlag level={row.priority.level} />
          </span>
        )}
      </div>
      <span className="board-card__actions">
        <RowActions issueId={row.id} issueKey={row.key} changed={row.changed} actions={actions} />
      </span>
    </article>
  );
});
