import { describe, expect, it } from "vitest";
import { type Collapsible, OVERFLOW_ORDER, overflowing, SAVE_VIEW, TEXT_FILTER_MIN } from "./fit";

const WIDTHS: Record<Collapsible, number> = { columns: 100, subtasks: 100, filter: 100, group: 100, me: 100 };
/** Five items with their 8px gaps, plus the text filter at its minimum. */
const ALL_SHOWN = 5 * (100 + 8) + TEXT_FILTER_MIN;
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

  it("hides items in the order Columns, Subtasks, Filter, Group, Me as the bar narrows", () => {
    expect(OVERFLOW_ORDER).toEqual(["columns", "subtasks", "filter", "group", "me"]);
    expect(overflowing(565, WIDTHS)).toEqual(["columns", "subtasks"]);
    expect(overflowing(457, WIDTHS)).toEqual(["columns", "subtasks", "filter"]);
    expect(overflowing(349, WIDTHS)).toEqual(["columns", "subtasks", "filter", "group"]);
    expect(overflowing(241, WIDTHS)).toEqual(["columns", "subtasks", "filter", "group", "me"]);
  });

  it("counts the width of the More button once something is hidden", () => {
    const withoutColumns = 4 * (100 + 8) + TEXT_FILTER_MIN;

    expect(overflowing(withoutColumns + MORE, WIDTHS)).toEqual(["columns"]);
    expect(overflowing(withoutColumns + MORE - 1, WIDTHS)).toEqual(["columns", "subtasks"]);
  });

  it("brings items back in reverse order as the bar widens", () => {
    const hiddenAt = [241, 349, 457, 565, ALL_SHOWN - 1, ALL_SHOWN].map((available) => overflowing(available, WIDTHS));

    expect(hiddenAt.map((hidden) => hidden.length)).toEqual([5, 4, 3, 2, 1, 0]);
    expect(hiddenAt[1]).toEqual(["columns", "subtasks", "filter", "group"]);
  });

  it("counts an item that was never measured as just its gap", () => {
    expect(overflowing(5 * 8 + TEXT_FILTER_MIN, {})).toEqual([]);
    expect(overflowing(5 * 8 + TEXT_FILTER_MIN - 1, {})).not.toEqual([]);
  });

  it("keeps the other items when only one is measured and it fits", () => {
    expect(overflowing(100 + 5 * 8 + TEXT_FILTER_MIN, { columns: 100 })).toEqual([]);
  });

  describe("while the view has unsaved changes", () => {
    /** Save View (84) and its gap, and the "…" button (30) and its gap, which Revert View needs. */
    const DIRTY_EXTRA = SAVE_VIEW + 8 + MORE;

    it("reserves room for Save View and the More button even when nothing is hidden", () => {
      expect(overflowing(ALL_SHOWN + DIRTY_EXTRA, WIDTHS, [], true)).toEqual([]);
      expect(overflowing(ALL_SHOWN + DIRTY_EXTRA - 1, WIDTHS, [], true)).toEqual(["columns"]);
    });

    it("hides Columns where a clean bar of the same width would hide nothing", () => {
      const available = ALL_SHOWN + 64;

      expect(overflowing(available, WIDTHS)).toEqual([]);
      expect(overflowing(available, WIDTHS, [], true)).toEqual(["columns"]);
    });

    it("counts the More button once, however many items are hidden", () => {
      const withoutColumns = 4 * (100 + 8) + TEXT_FILTER_MIN + SAVE_VIEW + 8 + MORE;

      expect(overflowing(withoutColumns, WIDTHS, [], true)).toEqual(["columns"]);
      expect(overflowing(withoutColumns - 1, WIDTHS, [], true)).toEqual(["columns", "subtasks"]);
    });

    it("still ignores absent items", () => {
      const fourShown = 4 * (100 + 8) + TEXT_FILTER_MIN;

      expect(overflowing(fourShown + DIRTY_EXTRA, WIDTHS, ["subtasks"], true)).toEqual([]);
    });
  });

  describe("with absent items", () => {
    const FOUR_SHOWN = 4 * (100 + 8) + TEXT_FILTER_MIN;

    it("ignores an absent item's width and never lists it as hidden", () => {
      expect(overflowing(FOUR_SHOWN, WIDTHS, ["subtasks"])).toEqual([]);
      expect(overflowing(FOUR_SHOWN - 1, WIDTHS, ["subtasks"])).toEqual(["columns"]);
    });

    it("skips the absent item when the bar narrows further", () => {
      expect(overflowing(457, WIDTHS, ["subtasks"])).toEqual(["columns", "filter"]);
    });
  });
});
