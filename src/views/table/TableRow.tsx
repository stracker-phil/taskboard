import { type CSSProperties, memo, type ReactNode } from "react";
import type { Row } from "../../api";
import { CopyKey } from "../../components/issue/CopyKey";
import { StatusSquare } from "../../components/issue/StatusSquare";
import { CellValue, cellEditProps, type CellEditHandlers } from "../list/cellEdits";
import { columnDef } from "../list/columns";
import { LabelChips } from "../../components/issue/LabelChips";
import { keepTextUnselected, opensIssue } from "../list/opensIssue";
import { PickBox } from "../list/PickBox";
import type { PickKeys } from "../list/useRowClicks";
import { ChildCount, SubtaskIcon } from "../../components/issue/SubtaskIcon";
import type { Nesting } from "../list/flatten";
import { SubtaskCaret } from "../list/SubtaskCaret";
import { widthVar } from "./widths";
import { useFlash } from "../../lib/motion";

const cellWidth = (id: string): CSSProperties => ({ width: `var(${widthVar(id)})` });

interface CellProps {
  id: string;
  className: string;
  row: Row;
  edits: CellEditHandlers;
  onPick: (id: string) => void;
  children: ReactNode;
}

/** A column's cell at the table's width for it; the status, assignee and priority cells open their menus. */
function Cell({ id, className, row, edits, onPick, children }: CellProps) {
  const editing = cellEditProps(id, row, edits, onPick);
  const editable = "onClick" in editing;
  return (
    <span className={editable ? `${className} cell-edit` : className} role="gridcell" style={cellWidth(id)} {...editing}>
      <CellValue editable={editable}>{children}</CellValue>
    </span>
  );
}

interface Props {
  row: Row;
  columns: string[];
  /** Highlighted: the selected row, or a picked one while rows are picked. */
  selected: boolean;
  /** Its checkbox is ticked. */
  picked: boolean;
  /** Rows are picked: every checkbox shows. */
  picking: boolean;
  onSelect: (id: string, mods?: PickKeys) => void;
  onToggleBox: (id: string) => void;
  /** Selects without opening: a cell that opens a menu. */
  onPick: (id: string) => void;
  /** A double-click: always the full Issue view. */
  onOpen: (id: string) => void;
  /** The labels as tags after the summary. */
  inlineLabels?: boolean;
  /** The view shows subtasks under their parents: every title leaves room for a caret. */
  tree?: boolean;
  /** Its place in Hierarchy: its depth's indent, and a caret when it has children. */
  nesting?: Nesting;
  subtasksOpen?: boolean;
  onToggleSubtasks?: (id: string) => void;
  /** Its child count after the summary. */
  childChip?: boolean;
  /** The status, assignee and priority cells open their menus. */
  edits: CellEditHandlers;
}

/** One issue: key, summary, status, then the view's cells; the widths come from the table's variables. */
export const TableRow = memo(function TableRow(props: Props) {
  const { row, columns, inlineLabels = false, selected, picked, picking, onSelect, onToggleBox, onPick, onOpen, edits } = props;
  const { tree = false, nesting = null, subtasksOpen = true, onToggleSubtasks, childChip = false } = props;
  const flashing = useFlash(row.updatedAt);
  const className = ["table-row", tree && "table-row--tree", nesting?.parent && "table-row--parent", nesting && nesting.depth > 0 && "table-row--child", row.changed && "table-row--changed", flashing && "flash", selected && "table-row--selected"].filter(Boolean).join(" ");
  return (
    <div className={className} style={{ "--depth": nesting?.depth ?? 0 } as CSSProperties} role="row" aria-selected={selected} data-issue-id={row.id} onMouseDown={keepTextUnselected} onClick={(e) => onSelect(row.id, e)} onDoubleClick={(e) => opensIssue(e) && onOpen(row.id)}>
      <span className="table-row__pick">
        <PickBox issueKey={row.key} picked={picked} onToggle={() => onToggleBox(row.id)} className={picking ? "pick-box--shown" : ""} />
      </span>
      <span className="table-row__cell table-row__key" role="gridcell" style={cellWidth("key")}>
        <CopyKey issueKey={row.key} />
      </span>
      <span className="table-row__cell table-row__summary" role="gridcell">
        {nesting?.parent && <SubtaskCaret open={subtasksOpen} onToggle={() => onToggleSubtasks?.(row.id)} />}
        {row.isSubtask && <SubtaskIcon />}
        <span className="table-row__title">{row.summary}</span>
        {childChip && <ChildCount count={row.childCount} />}
        {inlineLabels && <LabelChips labels={row.labels} className="row-labels" />}
      </span>
      <Cell id="status" className="table-row__cell table-row__status" row={row} edits={edits} onPick={onPick}>
        <StatusSquare style={row.style} />
        <span className="status-name">{row.statusName}</span>
      </Cell>
      {columns.map((id) => (
        <Cell key={id} id={id} className="table-row__cell" row={row} edits={edits} onPick={onPick}>
          {columnDef(id).cell(row)}
        </Cell>
      ))}
    </div>
  );
});
