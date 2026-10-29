import type { Row } from "../../api";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { columnDef } from "../list/columns";
import { LabelChips } from "../../components/issue/LabelChips";
import { opensIssue } from "../list/opensIssue";

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
}

/** One issue: key, summary, status, then the view's cells. */
export function TableRow({ row, columns, widthOf, inlineLabels = false, selected, onSelect, onOpen }: Props) {
  const className = ["table-row", row.changed && "table-row--changed", selected && "table-row--selected"].filter(Boolean).join(" ");
  return (
    <div className={className} role="row" aria-selected={selected} data-issue-id={row.id} onClick={() => onSelect(row.id)} onDoubleClick={(e) => opensIssue(e) && onOpen(row.id)}>
      <span className="table-row__cell table-row__key" role="gridcell" style={{ width: widthOf("key") }}>
        {row.key}
      </span>
      <span className="table-row__cell table-row__summary" role="gridcell">
        <span className="table-row__title">{row.summary}</span>
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
