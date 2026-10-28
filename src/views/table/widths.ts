import { columnDef } from "../list/columns";

export const SUMMARY_MIN_WIDTH = 240;
export const MIN_COLUMN_WIDTH = 48;
const FIXED: Record<string, number> = { key: 90, status: 130 };

/** A column's width: the one the owner dragged it to, else its default. */
export function columnWidth(id: string, widths: Record<string, number>): number {
  return widths[id] ?? FIXED[id] ?? columnDef(id).width;
}
