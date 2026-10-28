import { describe, expect, it } from "vitest";
import { group, groupOf, row, viewData } from "../../test/issues";
import { flatten, GROUP_GAP, GROUP_HEIGHT, itemHeight, ROW_HEIGHT } from "./flatten";

const A = row({ id: "1", key: "NW-1" });
const B = row({ id: "2", key: "NW-2" });
const C = row({ id: "3", key: "NW-3" });

const data = () => viewData([groupOf([A, B], { key: "g1" }), groupOf([C], { key: "g2" })], [A, B, C]);
const kinds = (items: ReturnType<typeof flatten>) => items.map((i) => (i.kind === "group" ? `group:${i.group.key}` : `row:${i.row.id}`));

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

  it("has no headers and shows every row when not grouped, even if the callback says closed", () => {
    const ungrouped = { ...data(), groupBy: "none" as const };

    expect(kinds(flatten(ungrouped, () => false))).toEqual(["row:1", "row:2", "row:3"]);
  });

  it("skips row ids that have no row", () => {
    const missing = viewData([group({ key: "g", rowIds: ["1", "99"] })], [A]);

    expect(kinds(flatten(missing, () => true))).toEqual(["group:g", "row:1"]);
  });
});

describe("itemHeight", () => {
  it("gives rows their height and later groups an extra gap", () => {
    const [first, , , second] = flatten(data(), () => true);

    expect(itemHeight({ kind: "row", row: A })).toBe(ROW_HEIGHT);
    expect(itemHeight(first)).toBe(GROUP_HEIGHT);
    expect(itemHeight(second)).toBe(GROUP_HEIGHT + GROUP_GAP);
  });
});
