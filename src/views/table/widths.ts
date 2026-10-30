import { columnDef } from "../list/columns";

export const SUMMARY_MIN_WIDTH = 240;
export const MIN_COLUMN_WIDTH = 48;
/** The key's 102 hold its 12px inset on each side. */
const FIXED: Record<string, number> = { key: 102, status: 130 };

/** A column's width: the one the owner dragged it to, else its default. */
export function columnWidth(id: string, widths: Record<string, number>): number {
  return widths[id] ?? FIXED[id] ?? columnDef(id).width;
}

/** The table's CSS variable with a column's width, so a resize restyles the cells without re-rendering rows. */
export const widthVar = (id: string) => `--table-width-${id.replace(/[^\w-]/g, "_")}`;
