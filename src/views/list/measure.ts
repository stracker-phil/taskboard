import type { Cell, Row } from "../../api";

/** Width in px of `text` drawn in a CSS `font`. */
export type MeasureText = (text: string, font: string) => number;

const UI = `-apple-system, BlinkMacSystemFont, "SF Pro Text", system-ui, sans-serif`;
const MONO = `"SF Mono", ui-monospace, Menlo, monospace`;
export const FONT = {
  header: `600 11px ${UI}`,
  cell: `400 13px ${UI}`,
  chip: `500 12px ${UI}`,
  label: `400 12px ${UI}`,
  mono: `400 12px ${MONO}`,
  pill: `700 10px ${UI}`,
} as const;

/** Header letter-spacing (0.05em of 11px) per character. */
const HEADER_TRACKING = 0.55;
const AVATAR = 26;
const AVATAR_STEP = 18;
const CHIP_PADDING = 16;
const LABEL_GAP = 4;
const FLAG = 14;
const FLAG_GAP = 6;
export const EMPTY = "—";

/** How wide each cell kind draws; `columns.tsx` pairs one with every column. */
export const widthOf = {
  text: (measure: MeasureText, text: string | null, font: string = FONT.cell) => (text ? measure(text, font) : 0),
  avatars: (count: number) => (count > 0 ? AVATAR + AVATAR_STEP * (count - 1) : 0),
  chip: (measure: MeasureText, text: string | null) => (text ? measure(text, FONT.chip) + CHIP_PADDING : 0),
  priority: (measure: MeasureText, name: string | null) => (name ? FLAG + FLAG_GAP + measure(name, FONT.cell) : 0),
  /** The `activity` status pill: caps, 0.03em tracking, 10px padding and a 1px border on each side. */
  pill: (measure: MeasureText, name: string) => measure(name.toUpperCase(), FONT.pill) + 0.3 * name.length + 2 * 10 + 2,
  labels: (measure: MeasureText, names: string[]) =>
    names.reduce((sum, name, index) => sum + measure(name, FONT.label) + CHIP_PADDING + (index > 0 ? LABEL_GAP : 0), 0),
  custom: (measure: MeasureText, cell: Cell | undefined) => {
    if (!cell) return 0;
    return cell.kind === "person" ? AVATAR : measure(cell.value, FONT.cell);
  },
};

export interface MeasuredColumn {
  id: string;
  header: string;
  cellWidth: (row: Row, measure: MeasureText) => number;
}

/** No column takes more than this; a longer value (an epic's long name) truncates, so the title keeps room. */
export const MAX_COLUMN_WIDTH = 200;

/** Each column's width: as wide as its header or its widest cell among all loaded rows, so columns line up
 *  across groups, but at most `MAX_COLUMN_WIDTH`. */
export function columnWidths(rows: Row[], columns: MeasuredColumn[], measure: MeasureText): number[] {
  return columns.map((column) => {
    const header = measure(column.header.toUpperCase(), FONT.header) + HEADER_TRACKING * column.header.length;
    const widest = rows.reduce((max, row) => Math.max(max, column.cellWidth(row, measure)), 0);
    return Math.min(MAX_COLUMN_WIDTH, Math.ceil(Math.max(header, widest)));
  });
}

/** The grid template shared by every group header and row: the title takes what is left. */
export function gridTemplate(widths: number[]): string {
  return ["minmax(0, 1fr)", ...widths.map((w) => `${w}px`)].join(" ");
}

export function listColumns(rows: Row[], columns: MeasuredColumn[], measure: MeasureText): string {
  return gridTemplate(columnWidths(rows, columns, measure));
}

const TITLE_MIN = 240;
const COLUMN_GAP = 20;

/** How many columns, from the left, fit beside a title of at least `TITLE_MIN` in `available` px; the others
 *  hide (the peek narrows the list) and come back when there is room. */
export function fittingColumns(widths: number[], available: number): number {
  let used = TITLE_MIN;
  for (let index = 0; index < widths.length; index++) {
    used += widths[index] + COLUMN_GAP;
    if (used > available) return index;
  }
  return widths.length;
}

/** Measures with a 2D canvas and remembers each font and text; without a canvas (tests), estimates. */
export function canvasMeasure(): MeasureText {
  const context = typeof OffscreenCanvas === "undefined" ? null : new OffscreenCanvas(1, 1).getContext("2d");
  const cache = new Map<string, number>();
  return (text, font) => {
    const key = `${font}|${text}`;
    let width = cache.get(key);
    if (width === undefined) {
      if (context) {
        context.font = font;
        width = context.measureText(text).width;
      } else {
        width = text.length * 7;
      }
      cache.set(key, width);
    }
    return width;
  };
}
