import type { Row } from "../../api";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "../list/columns";

interface Props {
  row: Row;
  columns: string[];
  widthOf: (id: string) => number;
  selected: boolean;
  onSelect: (id: string) => void;
}

/** One issue: key, summary, status, then the view's cells. */
export function TableRow({ row, columns, widthOf, selected, onSelect }: Props) {
  const className = ["table-row", row.changed && "table-row--changed", selected && "table-row--selected"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} data-issue-id={row.id} onClick={() => onSelect(row.id)}>
      <span className="table-row__cell table-row__key" role="gridcell" style={{ width: widthOf("key") }}>
        {row.key}
      </span>
      <span className="table-row__cell table-row__summary" role="gridcell">
        {row.summary}
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
