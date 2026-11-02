import { memo } from "react";
import { CopyKey } from "../../components/issue/CopyKey";
import type { Row } from "../../api";
import { AvatarStack } from "../../components/issue/Avatar";
import { CommentCount } from "../../components/issue/CommentCount";
import { EpicChip } from "../../components/issue/EpicChip";
import { LabelChips } from "../../components/issue/LabelChips";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { RowActions, type RowActionHandlers } from "../list/RowActions";
import { keepTextUnselected, opensIssue } from "../list/opensIssue";
import { PickBox } from "../list/PickBox";
import type { PickKeys } from "../list/useRowClicks";
import { useFlash } from "../../lib/motion";
import { StatusSquareButton } from "../../writes/StatusSquareButton";

interface Props {
  row: Row;
  fields: string[];
  /** The labels as tags after the title, while they aren't a card field. */
  inlineLabels?: boolean;
  /** The status square before the title, when the columns aren't the statuses. */
  statusSquare?: boolean;
  /** Highlighted: the keyboard's current card. */
  selected?: boolean;
  /** Its checkbox is ticked. */
  picked?: boolean;
  onOpen: (id: string, mods?: PickKeys) => void;
  onToggleBox?: (id: string) => void;
  /** A ⌘ double-click: the issue in a new tab, shown with ⇧. */
  onOpenInTab?: (id: string, show: boolean) => void;
  actions: RowActionHandlers;
  /** Has an active bookmark. */
  pinned?: boolean;
}

/** A Board card: the summary first, always wrapped, the key in the bottom-right corner; changed cards are tinted. */
export const BoardCard = memo(function BoardCard({ row, fields, inlineLabels = false, statusSquare = false, selected = false, picked = false, onOpen, onToggleBox, onOpenInTab, actions, pinned = false }: Props) {
  const shows = (field: string) => fields.includes(field);
  const flashing = useFlash(row.updatedAt);
  const epic = shows("epic") && row.epic;
  const labels = shows("labels") && row.labels.length > 0;
  return (
    <article
      className={["board-card", row.changed && "board-card--changed", flashing && "flash", selected && "board-card--selected", picked && "board-card--picked"].filter(Boolean).join(" ")}
      aria-current={selected || picked || undefined}
      tabIndex={0}
      aria-label={`${row.key} ${row.summary}`}
      data-issue-id={row.id}
      onMouseDown={keepTextUnselected}
      onClick={(e) => onOpen(row.id, e)}
      onDoubleClick={(e) => e.metaKey && opensIssue(e) && onOpenInTab?.(row.id, e.shiftKey)}
      onKeyDown={(e) => {
        if (e.key === "Enter" && e.target === e.currentTarget) onOpen(row.id);
      }}
    >
      <p className="board-card__title">
        {statusSquare && <StatusSquareButton issue={row} changeStatus={actions.changeStatus} />}
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
      {onToggleBox && (
        <PickBox issueKey={row.key} picked={picked} onToggle={() => onToggleBox(row.id)} className="board-card__pick" />
      )}
      <span className="board-card__actions">
        <RowActions issueId={row.id} issueKey={row.key} changed={row.changed} actions={actions} pinned={pinned} />
      </span>
    </article>
  );
});
