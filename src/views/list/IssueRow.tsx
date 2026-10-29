import type { Row } from "../../api";
import { ChangeChip } from "../../components/issue/ChangeChip";
import { CommentCount } from "../../components/issue/CommentCount";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "./columns";
import type { CardPosition } from "./flatten";
import { RowActions, type RowActionHandlers } from "./RowActions";
import { LabelChips } from "../../components/issue/LabelChips";

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
  actions: RowActionHandlers;
}

export function IssueRow({ row, columns, position, wrap = false, inlineLabels = false, selected, onSelect, actions }: Props) {
  const className = ["list-row", `list-row--${position}`, wrap && "list-row--wrap", row.changed && "list-row--changed", selected && "list-row--selected"]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} data-issue-id={row.id} onClick={() => onSelect(row.id)}>
      <span className="list-row__name" role="gridcell">
        <StatusSquare style={row.style} />
        <span className="list-row__key">{row.key}</span>
        <span className="list-row__summary">{row.summary}</span>
        {inlineLabels && <LabelChips labels={row.labels} className="row-labels" />}
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
