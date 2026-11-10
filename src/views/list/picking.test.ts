import { describe, expect, it } from "vitest";
import { extendPick, groupPicks, groupRowIds, pickRange, toggleBox, toggleGroup, togglePick, viewOrder } from "./picking";

const ROWS = ["1", "2", "3", "4", "5"];

describe("togglePick (⌘-click)", () => {
  it("picks just the clicked row when nothing is picked or selected", () => {
    expect(togglePick(ROWS, [], "3", null)).toEqual(["3"]);
  });

  it("also picks the selected row with the first pick", () => {
    expect(togglePick(ROWS, [], "4", "2")).toEqual(["2", "4"]);
  });

  it("ignores the selected row once something is picked", () => {
    expect(togglePick(ROWS, ["1"], "3", "5")).toEqual(["1", "3"]);
  });

  it("unpicks a row that is picked", () => {
    expect(togglePick(ROWS, ["1", "3"], "3", "5")).toEqual(["1"]);
  });

  it("leaves nothing picked when the last pick is unpicked", () => {
    expect(togglePick(ROWS, ["3"], "3", null)).toEqual([]);
  });

  it("leaves nothing picked when the selected row itself is the first one clicked", () => {
    expect(togglePick(ROWS, [], "2", "2")).toEqual([]);
  });

  it("keeps the rows in their shown order whatever the click order", () => {
    expect(togglePick(ROWS, ["4"], "2", null)).toEqual(["2", "4"]);
  });
});

describe("toggleBox (checkbox)", () => {
  it("picks that row alone, without the selected row", () => {
    expect(toggleBox(ROWS, [], "3")).toEqual(["3"]);
  });

  it("adds to the picks in their shown order", () => {
    expect(toggleBox(ROWS, ["4"], "2")).toEqual(["2", "4"]);
  });

  it("unpicks a picked row and leaves the others", () => {
    expect(toggleBox(ROWS, ["2", "4"], "4")).toEqual(["2"]);
  });

  it("leaves nothing picked when the last pick is unticked", () => {
    expect(toggleBox(ROWS, ["2"], "2")).toEqual([]);
  });

  it("picks nothing in an empty list", () => {
    expect(toggleBox([], [], "2")).toEqual([]);
  });
});

describe("toggleGroup (a group's checkbox)", () => {
  it("picks every row of the group when nothing is picked", () => {
    expect(toggleGroup(ROWS, [], ["2", "3"])).toEqual(["2", "3"]);
  });

  it("picks every row of the group when only some of it is picked", () => {
    expect(toggleGroup(ROWS, ["2"], ["2", "3"])).toEqual(["2", "3"]);
  });

  it("unpicks every row of the group when all of it is picked", () => {
    expect(toggleGroup(ROWS, ["2", "3"], ["2", "3"])).toEqual([]);
  });

  it("keeps the picks of other groups when picking", () => {
    expect(toggleGroup(ROWS, ["5"], ["2", "3"])).toEqual(["2", "3", "5"]);
  });

  it("keeps the picks of other groups when unpicking", () => {
    expect(toggleGroup(ROWS, ["1", "2", "3"], ["2", "3"])).toEqual(["1"]);
  });

  it("returns the picks in the view's order whatever the order of the group's ids", () => {
    expect(toggleGroup(ROWS, ["5"], ["4", "2"])).toEqual(["2", "4", "5"]);
  });

  it("changes nothing for an empty group", () => {
    expect(toggleGroup(ROWS, ["1"], [])).toEqual(["1"]);
  });
});

describe("groupPicks", () => {
  it.each([
    { expected: "all", picked: ["2", "3", "5"] },
    { expected: "some", picked: ["3", "5"] },
    { expected: "none", picked: ["1", "5"] },
  ] as const)("returns $expected for the group [2, 3] when $picked are picked", ({ expected, picked }) => {
    expect(groupPicks(["2", "3"], new Set<string>(picked))).toBe(expected);
  });

  it("returns none for an empty group", () => {
    expect(groupPicks([], new Set(["1"]))).toBe("none");
  });
});

describe("viewOrder", () => {
  const rows = Object.fromEntries(ROWS.map((id) => [id, {}]));

  it("lists the rows of every group in order, those of a collapsed group or subtree included", () => {
    const groups = [{ rowIds: ["1", "2", "3"] }, { rowIds: ["4", "5"] }];

    expect(viewOrder(groups, rows)).toEqual(["1", "2", "3", "4", "5"]);
  });

  it("lists an issue once when it sits in several groups", () => {
    const groups = [{ rowIds: ["1", "2"] }, { rowIds: ["2", "3"] }];

    expect(viewOrder(groups, rows)).toEqual(["1", "2", "3"]);
  });

  it("skips ids that have no row", () => {
    expect(viewOrder([{ rowIds: ["1", "99", "2"] }], rows)).toEqual(["1", "2"]);
  });

  it("is empty without groups", () => {
    expect(viewOrder([], rows)).toEqual([]);
  });
});

describe("groupRowIds", () => {
  const rows = Object.fromEntries(ROWS.map((id) => [id, {}]));

  it("returns all of the group's ids that have a row", () => {
    expect(groupRowIds({ rowIds: ["3", "1", "2"] }, rows)).toEqual(["3", "1", "2"]);
  });

  it("skips ids missing from the rows", () => {
    expect(groupRowIds({ rowIds: ["1", "99", "2"] }, rows)).toEqual(["1", "2"]);
  });
});

describe("pickRange (⇧-click)", () => {
  it("picks the rows from the selected one down to the clicked one, both included", () => {
    expect(pickRange(ROWS, "2", "4")).toEqual(["2", "3", "4"]);
  });

  it("picks the same rows in shown order when the clicked row is above the selected one", () => {
    expect(pickRange(ROWS, "4", "2")).toEqual(["2", "3", "4"]);
  });

  it("picks the one row when the selected row is clicked", () => {
    expect(pickRange(ROWS, "3", "3")).toEqual(["3"]);
  });

  it("picks only the clicked row when nothing is selected", () => {
    expect(pickRange(ROWS, null, "4")).toEqual(["4"]);
  });

  it("picks only the clicked row when the selected row is not shown", () => {
    expect(pickRange(ROWS, "99", "4")).toEqual(["4"]);
  });

  it("picks nothing when the clicked row is not shown", () => {
    expect(pickRange(ROWS, "2", "99")).toEqual([]);
  });

  it("picks nothing in an empty list", () => {
    expect(pickRange([], null, "1")).toEqual([]);
  });
});

describe("extendPick (⇧J and ⇧K)", () => {
  it("picks the selected row and the one below it when nothing is picked", () => {
    expect(extendPick(ROWS, [], "2", 1)).toEqual({ picked: ["2", "3"], selected: "3" });
  });

  it("picks the selected row and the one above it going up", () => {
    expect(extendPick(ROWS, [], "2", -1)).toEqual({ picked: ["1", "2"], selected: "1" });
  });

  it("adds the row past the picks' bottom edge going down, not past the selected row", () => {
    expect(extendPick(ROWS, ["2", "3"], "2", 1)).toEqual({ picked: ["2", "3", "4"], selected: "4" });
  });

  it("adds the row past the picks' top edge going up", () => {
    expect(extendPick(ROWS, ["2", "3"], "3", -1)).toEqual({ picked: ["1", "2", "3"], selected: "1" });
  });

  it("measures the edge of picks that are not next to each other", () => {
    expect(extendPick(ROWS, ["1", "4"], "1", 1)).toEqual({ picked: ["1", "4", "5"], selected: "5" });
    expect(extendPick(ROWS, ["2", "5"], "5", -1)).toEqual({ picked: ["1", "2", "5"], selected: "1" });
  });

  it.each([1, -1] as const)("starts on the first row when nothing is picked or selected (step %i)", (step) => {
    expect(extendPick(ROWS, [], null, step)).toEqual({ picked: ["1"], selected: "1" });
  });

  it("stays at the bottom edge, selecting the last row again", () => {
    expect(extendPick(ROWS, ["4", "5"], "4", 1)).toEqual({ picked: ["4", "5"], selected: "5" });
  });

  it("stays at the top edge, selecting the first row again", () => {
    expect(extendPick(ROWS, ["1", "2"], "2", -1)).toEqual({ picked: ["1", "2"], selected: "1" });
  });

  it("picks just the last row when it is selected and nothing is picked", () => {
    expect(extendPick(ROWS, [], "5", 1)).toEqual({ picked: ["5"], selected: "5" });
  });

  it("changes nothing in an empty list", () => {
    expect(extendPick([], [], null, 1)).toEqual({ picked: [], selected: null });
  });
});
