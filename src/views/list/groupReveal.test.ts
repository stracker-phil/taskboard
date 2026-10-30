import { describe, expect, it } from "vitest";
import { group, row } from "../../test/issues";
import type { Item } from "./flatten";
import { revealedHeights } from "./groupReveal";

const header = (key: string): Item => ({ kind: "group", group: group({ key }), open: true, first: false, tab: true });
const line = (id: string, groupKey: string): Item => ({ kind: "row", row: row({ id, key: `NW-${id}` }), groupKey, position: "middle", nesting: null });
const fixed = (height: number) => () => height;

/** A header followed by four rows of the group `key`. */
function groupOfFour(key: string): Item[] {
  return [header(key), ...[1, 2, 3, 4].map((n) => line(`${key}-${n}`, key))];
}

describe("revealedHeights", () => {
  it("keeps every height when no group is moving", () => {
    const items = groupOfFour("a");
    expect(revealedHeights(items, {}, (i) => (i === 0 ? 30 : 44))).toEqual([30, 44, 44, 44, 44]);
  });

  it("shows whole rows of a group at factor 1", () => {
    expect(revealedHeights(groupOfFour("a"), { a: 1 }, fixed(44))).toEqual([44, 44, 44, 44, 44]);
  });

  it("hides all rows of a group at factor 0 while its header and other groups stay", () => {
    const items = [...groupOfFour("a"), ...groupOfFour("b")];
    const heights = revealedHeights(items, { a: 0 }, (i) => (items[i].kind === "group" ? 30 : 44));
    expect(heights).toEqual([30, 0, 0, 0, 0, 30, 44, 44, 44, 44]);
  });

  it("shows the top half of a half-open group: two rows whole, two hidden", () => {
    const heights = revealedHeights(groupOfFour("a"), { a: 0.5 }, (i) => (i === 0 ? 30 : 44));
    expect(heights).toEqual([30, 44, 44, 0, 0]);
  });

  it("cuts the row on the reveal edge partway", () => {
    const heights = revealedHeights(groupOfFour("a").slice(1, 4), { a: 0.5 }, fixed(44));
    expect(heights).toEqual([44, 22, 0]);
  });

  it("reveals rows of different heights from the top", () => {
    const items = groupOfFour("a").slice(1);
    const rowHeights = [44, 88, 44, 44];
    expect(revealedHeights(items, { a: 0.5 }, (i) => rowHeights[i])).toEqual([44, 66, 0, 0]);
  });

  it("moves two groups independently", () => {
    const items = [...groupOfFour("a"), ...groupOfFour("b")];
    const heights = revealedHeights(items, { a: 0.25, b: 0.75 }, (i) => (items[i].kind === "group" ? 30 : 44));
    expect(heights).toEqual([30, 44, 0, 0, 0, 30, 44, 44, 44, 0]);
  });
});
