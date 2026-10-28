import type { Row } from "../../api";
import { ChangeChip } from "../../components/issue/ChangeChip";
import { CommentCount } from "../../components/issue/CommentCount";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "./columns";

interface Props {
  row: Row;
  columns: string[];
  selected: boolean;
  onSelect: (id: string) => void;
}

export function IssueRow({ row, columns, selected, onSelect }: Props) {
  const className = ["list-row", row.changed && "list-row--changed", selected && "list-row--selected"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} onClick={() => onSelect(row.id)}>
      <span className="list-row__name" role="gridcell">
        <StatusSquare style={row.style} />
        <span className="list-row__key">{row.key}</span>
        <span className="list-row__summary">{row.summary}</span>
        {row.changeNote && <ChangeChip note={row.changeNote} />}
        <CommentCount count={row.commentCount} />
      </span>
      {columns.map((id) => {
        const column = columnDef(id);
        return (
          <span key={id} className="list-row__cell" role="gridcell" style={{ width: column.width }}>
            {column.cell(row)}
          </span>
        );
      })}
    </div>
  );
}
