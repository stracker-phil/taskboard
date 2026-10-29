import type { Row } from "../../api";
import { CopyKey } from "../../components/issue/CopyKey";
import { ChangeChip } from "../../components/issue/ChangeChip";
import { CommentCount } from "../../components/issue/CommentCount";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "./columns";
import type { CardPosition, Nesting } from "./flatten";
import { RowActions, type RowActionHandlers } from "./RowActions";
import { LabelChips } from "../../components/issue/LabelChips";
import { opensIssue } from "./opensIssue";
import { SubtaskCount, SubtaskIcon } from "../../components/issue/SubtaskIcon";
import { SubtaskCaret } from "./SubtaskCaret";

interface Props {
  row: Row;
  columns: string[];
  /** Its place in the group's card: decides the corners, edges and the separator above. */
  position: CardPosition;
  /** The title wraps onto more lines instead of truncating. */
  wrap?: boolean;
  /** The labels as tags after the title. */
  inlineLabels?: boolean;
  selected: boolean;
  onSelect: (id: string) => void;
  /** A double-click: always the full Issue view. */
  onOpen: (id: string) => void;
  actions: RowActionHandlers;
  /** Its place in Hierarchy: a parent's caret, a subtask's indent. */
  nesting?: Nesting;
  /** A parent's subtasks show below it. */
  subtasksOpen?: boolean;
  onToggleSubtasks?: (id: string) => void;
  /** The parent's subtask count after the title. */
  subtaskChip?: boolean;
}

export function IssueRow(props: Props) {
  const { row, columns, position, wrap = false, inlineLabels = false, selected, onSelect, onOpen, actions } = props;
  const { nesting = null, subtasksOpen = true, onToggleSubtasks, subtaskChip = false } = props;
  const className = ["list-row", `list-row--${position}`, wrap && "list-row--wrap", nesting === "child" && "list-row--child", row.changed && "list-row--changed", selected && "list-row--selected"]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} data-issue-id={row.id} onClick={() => onSelect(row.id)} onDoubleClick={(e) => opensIssue(e) && onOpen(row.id)}>
      <span className="list-row__name" role="gridcell">
        {nesting === "parent" && <SubtaskCaret open={subtasksOpen} onToggle={() => onToggleSubtasks?.(row.id)} />}
        <StatusSquare style={row.style} />
        <CopyKey className="list-row__key" issueKey={row.key} />
        {row.isSubtask && <SubtaskIcon />}
        <span className="list-row__title">
          <span className="list-row__summary">{row.summary}</span>
          {inlineLabels && <LabelChips labels={row.labels} className={wrap ? "row-labels row-labels--flow" : "row-labels"} />}
        </span>
        {subtaskChip && <SubtaskCount count={row.subtaskCount} />}
        {row.changeNote && <ChangeChip note={row.changeNote} />}
        <CommentCount count={row.commentCount} />
        <RowActions issueId={row.id} issueKey={row.key} changed={row.changed} actions={actions} />
      </span>
      {columns.map((id) => (
        <span key={id} className="list-row__cell" role="gridcell">
          {columnDef(id).cell(row)}
        </span>
      ))}
    </div>
  );
}
