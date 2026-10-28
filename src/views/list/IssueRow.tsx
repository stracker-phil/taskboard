import type { Row } from "../../api";
import { ChangeChip } from "../../components/issue/ChangeChip";
import { CommentCount } from "../../components/issue/CommentCount";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "./columns";
import type { CardPosition } from "./flatten";
import { RowActions, type RowActionHandlers } from "./RowActions";

interface Props {
  row: Row;
  columns: string[];
  /** Its place in the group's card: decides the corners, edges and the separator above. */
  position: CardPosition;
  selected: boolean;
  onSelect: (id: string) => void;
  actions: RowActionHandlers;
}

export function IssueRow({ row, columns, position, selected, onSelect, actions }: Props) {
  const className = ["list-row", `list-row--${position}`, row.changed && "list-row--changed", selected && "list-row--selected"]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} onClick={() => onSelect(row.id)}>
      <span className="list-row__name" role="gridcell">
        <StatusSquare style={row.style} />
        <span className="list-row__key">{row.key}</span>
        <span className="list-row__summary">{row.summary}</span>
        {row.changeNote && <ChangeChip note={row.changeNote} />}
        <CommentCount count={row.commentCount} />
        <RowActions issueKey={row.key} actions={actions} />
      </span>
      {columns.map((id) => (
        <span key={id} className="list-row__cell" role="gridcell">
          {columnDef(id).cell(row)}
        </span>
      ))}
    </div>
  );
}
