import { describe, expect, it } from "vitest";
import { type Collapsible, OVERFLOW_ORDER, overflowing, TEXT_FILTER_MIN } from "./fit";

const WIDTHS: Record<Collapsible, number> = { columns: 100, filter: 100, group: 100, me: 100 };
/** Four items with their 8px gaps, plus the text filter at its minimum. */
const ALL_SHOWN = 4 * (100 + 8) + TEXT_FILTER_MIN;
/** The "…" button (30) and its gap (8) appear once anything is hidden. */
const MORE = 38;

describe("overflowing", () => {
  it("hides nothing when everything fits", () => {
    expect(overflowing(ALL_SHOWN, WIDTHS)).toEqual([]);
    expect(overflowing(2000, WIDTHS)).toEqual([]);
  });

  it("hides Columns first when the bar is one pixel too narrow", () => {
    expect(overflowing(ALL_SHOWN - 1, WIDTHS)).toEqual(["columns"]);
  });

  it("hides items in the order Columns, Filter, Group, Me as the bar narrows", () => {
    expect(OVERFLOW_ORDER).toEqual(["columns", "filter", "group", "me"]);
    expect(overflowing(457, WIDTHS)).toEqual(["columns", "filter"]);
    expect(overflowing(349, WIDTHS)).toEqual(["columns", "filter", "group"]);
    expect(overflowing(241, WIDTHS)).toEqual(["columns", "filter", "group", "me"]);
  });

  it("counts the width of the More button once something is hidden", () => {
    const withoutColumns = 3 * (100 + 8) + TEXT_FILTER_MIN;

    expect(overflowing(withoutColumns + MORE, WIDTHS)).toEqual(["columns"]);
    expect(overflowing(withoutColumns + MORE - 1, WIDTHS)).toEqual(["columns", "filter"]);
  });

  it("brings items back in reverse order as the bar widens", () => {
    const hiddenAt = [241, 349, 457, ALL_SHOWN - 1, ALL_SHOWN].map((available) => overflowing(available, WIDTHS));

    expect(hiddenAt.map((hidden) => hidden.length)).toEqual([4, 3, 2, 1, 0]);
    expect(hiddenAt[1]).toEqual(["columns", "filter", "group"]);
  });

  it("counts an item that was never measured as just its gap", () => {
    expect(overflowing(4 * 8 + TEXT_FILTER_MIN, {})).toEqual([]);
    expect(overflowing(4 * 8 + TEXT_FILTER_MIN - 1, {})).not.toEqual([]);
  });

  it("keeps the other items when only one is measured and it fits", () => {
    expect(overflowing(100 + 4 * 8 + TEXT_FILTER_MIN, { columns: 100 })).toEqual([]);
  });
});
