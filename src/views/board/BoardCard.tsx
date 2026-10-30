import { memo } from "react";
import { CopyKey } from "../../components/issue/CopyKey";
import type { Row } from "../../api";
import { AvatarStack } from "../../components/issue/Avatar";
import { CommentCount } from "../../components/issue/CommentCount";
import { EpicChip } from "../../components/issue/EpicChip";
import { LabelChips } from "../../components/issue/LabelChips";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { RowActions, type RowActionHandlers } from "../list/RowActions";

interface Props {
  row: Row;
  fields: string[];
  /** The labels as tags after the title, while they aren't a card field. */
  inlineLabels?: boolean;
  /** The keyboard's current card. */
  selected?: boolean;
  onOpen: (id: string) => void;
  actions: RowActionHandlers;
  /** Has an active bookmark. */
  pinned?: boolean;
}

/** A Board card: the summary first, always wrapped, the key in the bottom-right corner; changed cards are tinted. */
export const BoardCard = memo(function BoardCard({ row, fields, inlineLabels = false, selected = false, onOpen, actions, pinned = false }: Props) {
  const shows = (field: string) => fields.includes(field);
  const epic = shows("epic") && row.epic;
  const labels = shows("labels") && row.labels.length > 0;
  return (
    <article
      className={["board-card", row.changed && "board-card--changed", selected && "board-card--selected"].filter(Boolean).join(" ")}
      aria-current={selected || undefined}
      tabIndex={0}
      aria-label={`${row.key} ${row.summary}`}
      data-issue-id={row.id}
      onClick={() => onOpen(row.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter" && e.target === e.currentTarget) onOpen(row.id);
      }}
    >
      <p className="board-card__title">
        <span className="board-card__summary">{row.summary}</span>
        {inlineLabels && <LabelChips labels={row.labels} className="row-labels row-labels--flow" />}
      </p>
      {(epic || labels) && (
        <div className="board-card__tags">
          {epic && <EpicChip epic={epic} />}
          {labels && <LabelChips labels={row.labels} className="board-card__labels" />}
        </div>
      )}
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
        {row.changed && <span className="board-card__dot" title="Changed since yesterday" />}
        <CopyKey className="board-card__key" issueKey={row.key} />
      </div>
      <span className="board-card__actions">
        <RowActions issueId={row.id} issueKey={row.key} changed={row.changed} actions={actions} pinned={pinned} />
      </span>
    </article>
  );
});
