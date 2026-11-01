import type { MouseEvent, ReactNode } from "react";
import type { RowActionHandlers } from "./RowActions";

export type CellEditHandlers = Pick<RowActionHandlers, "changeStatus" | "assignTo" | "changePriority">;

/** The menu each editable column opens, and the attribute the keys (D, A, ⇧P) anchor that menu to. */
const EDITS = {
  status: { handler: "changeStatus", anchor: "data-status-menu" },
  assignee: { handler: "assignTo", anchor: "data-assign-menu" },
  priority: { handler: "changePriority", anchor: "data-priority-menu" },
} as const;

/** Props that make a row's cell open its column's menu, while that kind of change to Jira is on; none for other
 *  cells. The click only selects the row (`onPick`: no preview, no Issue view), so the menu acts on what is
 *  selected; a double-click stays in the cell. */
export function cellEditProps(column: string, row: { id: string; key: string }, edits: CellEditHandlers, onPick: (id: string) => void) {
  const edit = column in EDITS ? EDITS[column as keyof typeof EDITS] : null;
  const open = edit ? edits[edit.handler] : null;
  if (!edit || !open) return {};
  return {
    [edit.anchor]: row.id,
    "aria-haspopup": "menu" as const,
    onClick: (e: MouseEvent<HTMLElement>) => {
      e.stopPropagation();
      onPick(row.id);
      open(row, e.currentTarget);
    },
    onDoubleClick: (e: MouseEvent<HTMLElement>) => e.stopPropagation(),
  };
}

/** An editable cell's value in the box its hover shades; other cells' values as they are. */
export function CellValue({ editable, children }: { editable: boolean; children: ReactNode }) {
  return editable ? <span className="cell-edit__value">{children}</span> : children;
}
