import type { ReactNode } from "react";
import type { Cell, Row } from "../../api";
import { Avatar, AvatarStack } from "../../components/issue/Avatar";
import { EpicChip } from "../../components/issue/EpicChip";
import { PriorityFlag } from "../../components/issue/PriorityFlag";

/** A list column: its width (docs/ux.md → List view) and how a row fills it. */
interface ColumnDef {
  width: number;
  cell: (row: Row) => ReactNode;
}

const EMPTY = "—";

const STANDARD: Record<string, ColumnDef> = {
  assignee: { width: 96, cell: (row) => <AvatarStack people={row.assignees} /> },
  epic: { width: 176, cell: (row) => row.epic && <EpicChip epic={row.epic} /> },
  release: { width: 96, cell: (row) => <span className="list-cell--mono">{row.release ?? EMPTY}</span> },
  priority: {
    width: 104,
    cell: (row) =>
      row.priority && (
        <span className="list-cell--priority">
          <PriorityFlag level={row.priority.level} />
          {row.priority.name}
        </span>
      ),
  },
  labels: {
    width: 120,
    cell: (row) => (
      <span className="list-cell--labels">
        {row.labels.map((label) => (
          <span key={label.name} className={`label-chip label-chip--${label.tint}`}>
            {label.name}
          </span>
        ))}
      </span>
    ),
  },
  sprint: { width: 100, cell: (row) => <span className="list-cell--secondary">{row.sprint ?? ""}</span> },
  due: { width: 80, cell: (row) => <span className="list-cell--tertiary">{row.due ?? EMPTY}</span> },
};

const CUSTOM_WIDTH = 90;

function customCell(cell: Cell | undefined): ReactNode {
  if (!cell) return null;
  if (cell.kind === "person") return <Avatar person={cell.value} />;
  return <span className="list-cell--secondary">{cell.value}</span>;
}

/** A standard column, or a custom field's by its Jira id. */
export function columnDef(id: string): ColumnDef {
  return STANDARD[id] ?? { width: CUSTOM_WIDTH, cell: (row) => customCell(row.custom[id]) };
}
