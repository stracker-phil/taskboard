import { describe, expect, it } from "vitest";
import { group, row } from "../../test/issues";
import { dragGroups } from "./dragGroups";
import type { Item } from "./flatten";

const header = (key: string, patch: { first?: boolean; tab?: boolean; open?: boolean } = {}): Item => ({
  kind: "group",
  group: group({ key, label: key.toUpperCase() }),
  open: patch.open ?? true,
  first: patch.first ?? false,
  tab: patch.tab ?? true,
});

/** Rows `<key>1` … `<key>n` of the group `key`, with positions `dragGroups` must recompute. */
const rowsOf = (key: string, count: number): Item[] =>
  Array.from({ length: count }, (_, i) => ({ kind: "row", row: row({ id: `${key}${i + 1}` }), groupKey: key, position: "first", nesting: null }));

const ids = (items: ReturnType<typeof dragGroups>[number]) => items.rows.map((item) => item.row.id);

describe("dragGroups", () => {
  it("lists every group in order, a closed group with its rows too", () => {
    const items = [header("a", { first: true }), ...rowsOf("a", 1), header("b", { open: false }), ...rowsOf("b", 2), header("c"), ...rowsOf("c", 1)];

    const groups = dragGroups(items, "a1", 2);

    expect(groups.map((g) => g.group.key)).toEqual(["a", "b", "c"]);
    expect(ids(groups[1])).toEqual(["b1", "b2"]);
  });

  it("passes on whether a group has a tab and whether it is the first", () => {
    const groups = dragGroups([header("a", { first: true, tab: false }), ...rowsOf("a", 1)], "a1", 2);

    expect(groups[0]).toMatchObject({ tab: false, first: true });
  });

  it("shows at most the given number of rows per group", () => {
    const groups = dragGroups([header("a"), ...rowsOf("a", 5)], "elsewhere", 2);

    expect(ids(groups[0])).toEqual(["a1", "a2"]);
  });

  it("keeps the dragged row where it is when it is among the shown rows", () => {
    const groups = dragGroups([header("a"), ...rowsOf("a", 5)], "a1", 2);

    expect(ids(groups[0])).toEqual(["a1", "a2"]);
  });

  it("lets the dragged row replace the last shown row when it would be hidden", () => {
    const groups = dragGroups([header("a"), ...rowsOf("a", 5)], "a5", 2);

    expect(ids(groups[0])).toEqual(["a1", "a5"]);
  });

  it("does not change another group for a dragged row that sits elsewhere", () => {
    const groups = dragGroups([header("a"), ...rowsOf("a", 3), header("b"), ...rowsOf("b", 4)], "b4", 2);

    expect(ids(groups[0])).toEqual(["a1", "a2"]);
    expect(ids(groups[1])).toEqual(["b1", "b4"]);
  });

  describe("more", () => {
    it.each([
      ["every row shows", 2, 0],
      ["fewer rows than shown", 1, 0],
      ["rows are hidden", 5, 3],
    ])("counts the hidden rows when %s", (_name, count, more) => {
      const groups = dragGroups([header("a"), ...rowsOf("a", count)], "elsewhere", 2);

      expect(groups[0].more).toBe(more);
    });

    it("still counts the row the dragged row displaced", () => {
      const groups = dragGroups([header("a"), ...rowsOf("a", 5)], "a5", 2);

      expect(groups[0].more).toBe(3);
    });
  });

  describe("card positions", () => {
    it("marks a single row as the only one", () => {
      const groups = dragGroups([header("a"), ...rowsOf("a", 1)], "a1", 2);

      expect(groups[0].rows.map((r) => r.position)).toEqual(["only"]);
    });

    it("recomputes them for the shown rows rather than the group's", () => {
      const groups = dragGroups([header("a"), ...rowsOf("a", 5)], "elsewhere", 2);

      expect(groups[0].rows.map((r) => r.position)).toEqual(["first", "last"]);
    });

    it("gives rows between the first and the last the middle position", () => {
      const groups = dragGroups([header("a"), ...rowsOf("a", 4)], "elsewhere", 4);

      expect(groups[0].rows.map((r) => r.position)).toEqual(["first", "middle", "middle", "last"]);
    });

    it("gives the displacing dragged row the last position", () => {
      const groups = dragGroups([header("a"), ...rowsOf("a", 4)], "a4", 3);

      expect(groups[0].rows.map((r) => [r.row.id, r.position])).toEqual([["a1", "first"], ["a2", "middle"], ["a4", "last"]]);
    });
  });

  it("gives a group without rows no rows and nothing more", () => {
    const groups = dragGroups([header("a"), header("b"), ...rowsOf("b", 1)], "b1", 2);

    expect(groups[0]).toMatchObject({ rows: [], more: 0 });
  });

  it("returns nothing for no items", () => {
    expect(dragGroups([], "a1", 2)).toEqual([]);
  });
});
