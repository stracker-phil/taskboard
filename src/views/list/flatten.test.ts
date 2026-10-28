import { describe, expect, it } from "vitest";
import { group, groupOf, row, viewData } from "../../test/issues";
import { flatten, GROUP_GAP, GROUP_HEIGHT, itemGroupKey, itemHeight, ROW_HEIGHT } from "./flatten";

const A = row({ id: "1", key: "NW-1" });
const B = row({ id: "2", key: "NW-2" });
const C = row({ id: "3", key: "NW-3" });
const D = row({ id: "4", key: "NW-4" });

const data = () => viewData([groupOf([A, B], { key: "g1" }), groupOf([C], { key: "g2" })], [A, B, C]);
const kinds = (items: ReturnType<typeof flatten>) => items.map((i) => (i.kind === "group" ? `group:${i.group.key}` : `row:${i.row.id}`));
const positions = (items: ReturnType<typeof flatten>) => items.flatMap((i) => (i.kind === "row" ? [i.position] : []));

describe("flatten", () => {
  it("lists each group header followed by its rows when all are open", () => {
    expect(kinds(flatten(data(), () => true))).toEqual(["group:g1", "row:1", "row:2", "group:g2", "row:3"]);
  });

  it("gives a closed group only its header", () => {
    const items = flatten(data(), (g) => g.key !== "g1");

    expect(kinds(items)).toEqual(["group:g1", "group:g2", "row:3"]);
    expect(items[0]).toMatchObject({ kind: "group", open: false });
  });

  it("marks only the first group as first", () => {
    const headers = flatten(data(), () => true).filter((i) => i.kind === "group");

    expect(headers.map((h) => h.first)).toEqual([true, false]);
  });

  it("gives every header of a grouped view a tab", () => {
    const headers = flatten(data(), () => true).filter((i) => i.kind === "group");

    expect(headers.map((h) => h.tab)).toEqual([true, true]);
  });

  it("tags each row with the key of its group", () => {
    const rows = flatten(data(), () => true).filter((i) => i.kind === "row");

    expect(rows.map((r) => r.groupKey)).toEqual(["g1", "g1", "g2"]);
  });

  it.each([
    [[A], ["only"]],
    [[A, B], ["first", "last"]],
    [[A, B, C], ["first", "middle", "last"]],
    [[A, B, C, D], ["first", "middle", "middle", "last"]],
  ])("positions the rows of a group (case %#) within its card", (rows, expected) => {
    const items = flatten(viewData([groupOf(rows)], rows), () => true);

    expect(positions(items)).toEqual(expected);
  });

  it("restarts the card positions in every group", () => {
    expect(positions(flatten(data(), () => true))).toEqual(["first", "last", "only"]);
  });

  describe("when not grouped", () => {
    const ungrouped = () => ({ ...viewData([groupOf([A, B, C], { key: "all" })], [A, B, C]), groupBy: "none" as const });

    it("has one tabless, open header before the rows, even if the callback says closed", () => {
      const items = flatten(ungrouped(), () => false);

      expect(kinds(items)).toEqual(["group:all", "row:1", "row:2", "row:3"]);
      expect(items[0]).toMatchObject({ kind: "group", open: true, first: true, tab: false });
    });

    it("lays the rows out as one card", () => {
      expect(positions(flatten(ungrouped(), () => false))).toEqual(["first", "middle", "last"]);
    });
  });

  it("skips row ids that have no row", () => {
    const missing = viewData([group({ key: "g", rowIds: ["1", "99"] })], [A]);

    const items = flatten(missing, () => true);

    expect(kinds(items)).toEqual(["group:g", "row:1"]);
    expect(positions(items)).toEqual(["only"]);
  });
});

describe("itemHeight", () => {
  it("gives rows their height and later groups an extra gap", () => {
    const [first, row1, , second] = flatten(data(), () => true);

    expect(itemHeight(row1)).toBe(ROW_HEIGHT);
    expect(itemHeight(first)).toBe(GROUP_HEIGHT);
    expect(itemHeight(second)).toBe(GROUP_HEIGHT + GROUP_GAP);
  });
});

describe("itemGroupKey", () => {
  it("names the group of a header and of its rows alike", () => {
    const [header, first, , nextHeader] = flatten(data(), () => true);

    expect([header, first, nextHeader].map(itemGroupKey)).toEqual(["g1", "g1", "g2"]);
  });
});
