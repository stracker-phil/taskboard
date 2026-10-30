import { describe, expect, it } from "vitest";
import { type Collapsible, DRAFT_ACTIONS, OVERFLOW_ORDER, overflowing, TEXT_FILTER_MIN } from "./fit";

const WIDTHS: Record<Collapsible, number> = { columns: 100, subtasks: 100, filter: 100, group: 100, me: 100 };
/** The room Revert View and Save View always keep, and the gap before them. */
const RESERVED = DRAFT_ACTIONS + 8;
/** `count` items with their 8px gaps, plus the text filter at its minimum and the draft actions' room. */
const room = (count: number) => count * (100 + 8) + TEXT_FILTER_MIN + RESERVED;
const ALL_SHOWN = room(5);
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

  it("reserves the draft actions' room even though nothing is drafted", () => {
    const withoutReserve = 5 * (100 + 8) + TEXT_FILTER_MIN;

    expect(overflowing(withoutReserve, WIDTHS)).not.toEqual([]);
    expect(overflowing(withoutReserve + RESERVED, WIDTHS)).toEqual([]);
  });

  it("hides items in the order Columns, Subtasks, Filter, Group, Me as the bar narrows", () => {
    expect(OVERFLOW_ORDER).toEqual(["columns", "subtasks", "filter", "group", "me"]);
    expect(overflowing(room(4) + MORE - 1, WIDTHS)).toEqual(["columns", "subtasks"]);
    expect(overflowing(room(3) + MORE - 1, WIDTHS)).toEqual(["columns", "subtasks", "filter"]);
    expect(overflowing(room(2) + MORE - 1, WIDTHS)).toEqual(["columns", "subtasks", "filter", "group"]);
    expect(overflowing(room(1) + MORE - 1, WIDTHS)).toEqual(["columns", "subtasks", "filter", "group", "me"]);
  });

  it("counts the width of the More button once something is hidden", () => {
    expect(overflowing(room(4) + MORE, WIDTHS)).toEqual(["columns"]);
    expect(overflowing(room(4) + MORE - 1, WIDTHS)).toEqual(["columns", "subtasks"]);
  });

  it("brings items back in reverse order as the bar widens", () => {
    const hiddenAt = [room(1) + MORE - 1, room(2) + MORE - 1, room(3) + MORE - 1, room(4) + MORE - 1, ALL_SHOWN - 1, ALL_SHOWN].map((available) =>
      overflowing(available, WIDTHS),
    );

    expect(hiddenAt.map((hidden) => hidden.length)).toEqual([5, 4, 3, 2, 1, 0]);
    expect(hiddenAt[1]).toEqual(["columns", "subtasks", "filter", "group"]);
  });

  it("counts an item that was never measured as just its gap", () => {
    expect(overflowing(5 * 8 + TEXT_FILTER_MIN + RESERVED, {})).toEqual([]);
    expect(overflowing(5 * 8 + TEXT_FILTER_MIN + RESERVED - 1, {})).not.toEqual([]);
  });

  it("keeps the other items when only one is measured and it fits", () => {
    expect(overflowing(100 + 5 * 8 + TEXT_FILTER_MIN + RESERVED, { columns: 100 })).toEqual([]);
  });

  describe("with absent items", () => {
    const FOUR_SHOWN = room(4);

    it("ignores an absent item's width and never lists it as hidden", () => {
      expect(overflowing(FOUR_SHOWN, WIDTHS, ["subtasks"])).toEqual([]);
      expect(overflowing(FOUR_SHOWN - 1, WIDTHS, ["subtasks"])).toEqual(["columns"]);
    });

    it("skips the absent item when the bar narrows further", () => {
      expect(overflowing(457 + RESERVED, WIDTHS, ["subtasks"])).toEqual(["columns", "filter"]);
    });
  });
});
