import { type CSSProperties, Fragment } from "react";
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
import { ChildCount, SubtaskIcon } from "../../components/issue/SubtaskIcon";
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
  /** Its place in Hierarchy: its depth's indent, and a caret when it has children. */
  nesting?: Nesting;
  /** Its subtree shows below it. */
  subtasksOpen?: boolean;
  onToggleSubtasks?: (id: string) => void;
  /** Its child count after the title. */
  childChip?: boolean;
}

export function IssueRow(props: Props) {
  const { row, columns, position, wrap = false, inlineLabels = false, selected, onSelect, onOpen, actions } = props;
  const { nesting = null, subtasksOpen = true, onToggleSubtasks, childChip = false } = props;
  const nested = (nesting?.depth ?? 0) > 0;
  const className = ["list-row", `list-row--${position}`, wrap && "list-row--wrap", nested && "list-row--child", row.changed && "list-row--changed", selected && "list-row--selected"]
    .filter(Boolean)
    .join(" ");
  // A wrapping title takes the chips into its flow, each after a space that vanishes at a line start; on one
  // line they follow the title and the summary is cut first.
  const chips = [
    childChip && <ChildCount key="children" count={row.childCount} />,
    row.changeNote && <ChangeChip key="change" note={row.changeNote} />,
    <CommentCount key="comments" count={row.commentCount} />,
  ];
  const flowingChips = chips.map((chip) => chip && <Fragment key={chip.key}> {chip}</Fragment>);
  const indentStyle = { "--depth": nesting?.depth ?? 0 } as CSSProperties;
  const caretClass = ["list-row-caret", nested && "list-row-caret--nested", wrap && "list-row-caret--wrap"].filter(Boolean).join(" ");
  return (
    <>
      {nesting?.parent && <SubtaskCaret className={caretClass} style={indentStyle} open={subtasksOpen} onToggle={() => onToggleSubtasks?.(row.id)} />}
      <div className={className} style={indentStyle} role="row" aria-selected={selected} data-issue-id={row.id} onClick={() => onSelect(row.id)} onDoubleClick={(e) => opensIssue(e) && onOpen(row.id)}>
        <span className="list-row__name" role="gridcell">
          <StatusSquare style={row.style} />
          <span className="list-row__title">
            <CopyKey className="list-row__key" issueKey={row.key} />
            {row.isSubtask && <SubtaskIcon />}
            <span className="list-row__summary">{row.summary}</span>
            {inlineLabels && <LabelChips labels={row.labels} className={wrap ? "row-labels row-labels--flow" : "row-labels"} />}
            {wrap && flowingChips}
          </span>
          {!wrap && chips}
          <RowActions issueId={row.id} issueKey={row.key} changed={row.changed} actions={actions} />
        </span>
        {columns.map((id) => (
          <span key={id} className="list-row__cell" role="gridcell">
            {columnDef(id).cell(row)}
          </span>
        ))}
      </div>
    </>
  );
}
