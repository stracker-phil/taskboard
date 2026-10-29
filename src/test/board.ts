import type { BoardColumn, BoardData, Row } from "../api/types";

/** The "In Progress" column of the "Website relaunch" board, expanded and empty; override per test. */
export function boardColumn(patch: Partial<BoardColumn> = {}): BoardColumn {
  return { key: "3", name: "In Progress", mark: { kind: "status", style: "progress" }, style: "progress", collapsed: false, ids: [], ...patch };
}

/** A board of these columns and the rows they point at. */
export function boardData(columns: BoardColumn[], rows: Row[] = [], patch: Partial<BoardData> = {}): BoardData {
  return { columns, rows: Object.fromEntries(rows.map((r) => [r.id, r])), cardFields: [], ...patch };
}
