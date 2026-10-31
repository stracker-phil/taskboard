import { describe, expect, it } from "vitest";
import { group, row } from "../../test/issues";
import type { Item } from "./flatten";
import { groupSpans } from "./dropSpans";

const header = (key: string, tab = true): Item => ({ kind: "group", group: group({ key }), open: true, first: false, tab });
const issue = (id: string, groupKey: string): Item => ({ kind: "row", row: row({ id }), groupKey, position: "middle", nesting: null });
const at = (start: number, end: number) => ({ start, end });

describe("groupSpans", () => {
  it("starts a group's span at its first row, leaving the header clear, and reaches 28px into the gap before the next group", () => {
    const items = [header("a"), issue("1", "a"), issue("2", "a"), header("b"), issue("3", "b")];
    const measured = [at(0, 30), at(30, 74), at(74, 118), at(118, 192), at(192, 236)];

    const spans = groupSpans(items, measured, 236);

    expect(spans.get("a")).toEqual({ top: 30, height: 118 - 30 + 28, compact: false });
  });

  it("stops the last group's span at the end of its rows", () => {
    const items = [header("a"), issue("1", "a"), header("b"), issue("2", "b")];
    const measured = [at(0, 30), at(30, 74), at(74, 148), at(148, 192)];

    expect(groupSpans(items, measured, 192).get("b")).toEqual({ top: 148, height: 44, compact: false });
  });

  it("never reaches past the total height", () => {
    const items = [header("a"), issue("1", "a"), header("b")];
    const measured = [at(0, 30), at(30, 74), at(74, 148)];

    expect(groupSpans(items, measured, 90).get("a")).toEqual({ top: 30, height: 60, compact: false });
  });

  it("makes a closed group compact, covering its 30px header row", () => {
    const items = [header("a"), header("b"), issue("1", "b")];
    const measured = [at(0, 30), at(30, 104), at(104, 148)];

    expect(groupSpans(items, measured, 148).get("a")).toEqual({ top: 0, height: 30 + 28, compact: true });
  });

  it("starts an ungrouped view's items at the first row", () => {
    const items = [header("all", false), issue("1", "all"), issue("2", "all")];
    const measured = [at(0, 30), at(30, 74), at(74, 118)];

    expect(groupSpans(items, measured, 118).get("all")).toEqual({ top: 30, height: 88, compact: false });
  });

  it("starts at the first row when a group's items come without a header", () => {
    const items = [issue("1", "a"), issue("2", "a")];
    const measured = [at(0, 44), at(44, 88)];

    expect(groupSpans(items, measured, 88).get("a")).toEqual({ top: 0, height: 88, compact: false });
  });
});
