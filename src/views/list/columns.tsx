import type { ReactNode } from "react";
import type { Cell, Row } from "../../api";
import { Avatar, AvatarStack } from "../../components/issue/Avatar";
import { EpicChip } from "../../components/issue/EpicChip";
import { PriorityFlag } from "../../components/issue/PriorityFlag";
import { StatusPill } from "../../components/issue/StatusPill";
import { STYLE_NAMES } from "../../components/issue/styleNames";
import { EMPTY, FONT, type MeasureText, widthOf } from "./measure";
import { LabelChips } from "../../components/issue/LabelChips";

/** A view column: how a row fills it, how wide that draws (the List sizes columns to their content), and the
 *  Table's starting width. */
interface ColumnDef {
  width: number;
  cell: (row: Row) => ReactNode;
  measure: (row: Row, measure: MeasureText) => number;
}

const STANDARD: Record<string, ColumnDef> = {
  status: {
    width: 130,
    cell: (row) => <StatusPill style={row.style} name={row.statusName} variant="activity" />,
    measure: (row, m) => widthOf.pill(m, row.statusName),
  },
  statusType: {
    width: 120,
    cell: (row) => <StatusPill style={row.style} name={STYLE_NAMES[row.style]} variant="activity" />,
    measure: (row, m) => widthOf.pill(m, STYLE_NAMES[row.style]),
  },
  assignee: {
    width: 96,
    cell: (row) => <AvatarStack people={row.assignees} />,
    measure: (row) => widthOf.avatars(row.assignees.length),
  },
  epic: {
    width: 176,
    cell: (row) => row.epic && <EpicChip epic={row.epic} />,
    measure: (row, m) => widthOf.chip(m, row.epic?.name ?? null),
  },
  release: {
    width: 96,
    cell: (row) => <span className="list-cell--mono">{row.release ?? EMPTY}</span>,
    measure: (row, m) => widthOf.text(m, row.release ?? EMPTY, FONT.mono),
  },
  priority: {
    width: 104,
    cell: (row) =>
      row.priority && (
        <span className="list-cell--priority">
          <PriorityFlag level={row.priority.level} />
          {row.priority.name}
        </span>
      ),
    measure: (row, m) => widthOf.priority(m, row.priority?.name ?? null),
  },
  labels: {
    width: 120,
    cell: (row) => <LabelChips labels={row.labels} className="list-cell--labels" />,
    measure: (row, m) => widthOf.labels(m, row.labels.map((l) => l.name)),
  },
  sprint: {
    width: 100,
    cell: (row) => <span className="list-cell--secondary">{row.sprint ?? ""}</span>,
    measure: (row, m) => widthOf.text(m, row.sprint),
  },
  due: {
    width: 80,
    cell: (row) => <span className="list-cell--tertiary">{row.due ?? EMPTY}</span>,
    measure: (row, m) => widthOf.text(m, row.due ?? EMPTY),
  },
};

const CUSTOM_WIDTH = 90;

function customCell(cell: Cell | undefined): ReactNode {
  if (!cell) return null;
  if (cell.kind === "person") return <Avatar person={cell.value} />;
  return <span className="list-cell--secondary">{cell.value}</span>;
}

/** A standard column, or a custom field's by its Jira id. */
export function columnDef(id: string): ColumnDef {
  return (
    STANDARD[id] ?? {
      width: CUSTOM_WIDTH,
      cell: (row) => customCell(row.custom[id]),
      measure: (row, m) => widthOf.custom(m, row.custom[id]),
    }
  );
}
