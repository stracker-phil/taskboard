import type { Row } from "../../api";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "../list/columns";
import { LabelChips } from "../../components/issue/LabelChips";
import { opensIssue } from "../list/opensIssue";
import { SubtaskCount, SubtaskIcon } from "../../components/issue/SubtaskIcon";
import type { Nesting } from "../list/flatten";
import { SubtaskCaret } from "../list/SubtaskCaret";

interface Props {
  row: Row;
  columns: string[];
  widthOf: (id: string) => number;
  selected: boolean;
  onSelect: (id: string) => void;
  /** A double-click: always the full Issue view. */
  onOpen: (id: string) => void;
  /** The labels as tags after the summary. */
  inlineLabels?: boolean;
  /** The view shows subtasks under their parents: every title leaves room for a parent's caret. */
  tree?: boolean;
  /** Its place in Hierarchy: a parent's caret, a subtask's indent. */
  nesting?: Nesting;
  subtasksOpen?: boolean;
  onToggleSubtasks?: (id: string) => void;
  /** The parent's subtask count after the summary. */
  subtaskChip?: boolean;
}

/** One issue: key, summary, status, then the view's cells. */
export function TableRow(props: Props) {
  const { row, columns, widthOf, inlineLabels = false, selected, onSelect, onOpen } = props;
  const { tree = false, nesting = null, subtasksOpen = true, onToggleSubtasks, subtaskChip = false } = props;
  const className = ["table-row", tree && "table-row--tree", nesting === "parent" && "table-row--parent", nesting === "child" && "table-row--child", row.changed && "table-row--changed", selected && "table-row--selected"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} data-issue-id={row.id} onClick={() => onSelect(row.id)} onDoubleClick={(e) => opensIssue(e) && onOpen(row.id)}>
      <span className="table-row__cell table-row__key" role="gridcell" style={{ width: widthOf("key") }}>
        {row.key}
      </span>
      <span className="table-row__cell table-row__summary" role="gridcell">
        {nesting === "parent" && <SubtaskCaret open={subtasksOpen} onToggle={() => onToggleSubtasks?.(row.id)} />}
        {row.isSubtask && <SubtaskIcon />}
        <span className="table-row__title">{row.summary}</span>
        {subtaskChip && <SubtaskCount count={row.subtaskCount} />}
        {inlineLabels && <LabelChips labels={row.labels} className="row-labels" />}
      </span>
      <span className="table-row__cell table-row__status" role="gridcell" style={{ width: widthOf("status") }}>
        <StatusSquare style={row.style} />
        {row.statusName}
      </span>
      {columns.map((id) => (
        <span key={id} className="table-row__cell" role="gridcell" style={{ width: widthOf(id) }}>
          {columnDef(id).cell(row)}
        </span>
      ))}
    </div>
  );
}
