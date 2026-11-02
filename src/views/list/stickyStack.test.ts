import { describe, expect, it } from "vitest";
import { outline, spansOf } from "../../test/items";
import type { Item } from "./flatten";
import { stackAbove, stickyStack } from "./stickyStack";

const H = 40;
const evenly = () => H;

/** Everything is 40 high, so item `i` lies between `40 * i` and `40 * (i + 1)`. */
const stackAt = (items: Item[], top: number) => stickyStack(items, spansOf(items, evenly), top, evenly);

describe("stickyStack", () => {
  const tree = outline("#first", "Root", "-Child", "--Leaf", "--Leaf2", "-Child2", "-Child3", "Root2", "#second", "Lone");

  it("holds nothing at scroll 0", () => {
    expect(stackAt(tree, 0)).toEqual([]);
  });

  it("holds nothing while the next group's header is still at the line", () => {
    expect(stackAt(tree, 8 * H)).toEqual([]);
  });

  it("holds the group header, then each ancestor of the rows under the line, outermost first", () => {
    expect(stackAt(tree, 70)).toEqual([
      { index: 0, top: 0, height: H },
      { index: 1, top: H, height: H },
      { index: 2, top: 2 * H, height: H },
    ]);
  });

  it("holds only the group header while top-level leaves are under the line", () => {
    const leaves = outline("#first", "A", "B", "C", "#second", "D");

    expect(stackAt(leaves, 50)).toEqual([{ index: 0, top: 0, height: H }]);
  });

  it("pushes a child up as its own subtree ends, leaving its parent in place", () => {
    expect(stackAt(tree, 100)).toEqual([
      { index: 0, top: 0, height: H },
      { index: 1, top: H, height: H },
      { index: 2, top: 60, height: H },
    ]);
  });

  it("slides entries whose subtrees end together up as one block", () => {
    const nested = outline("#first", "Root", "-Child", "--Leaf", "--Leaf2", "#second", "Lone");

    expect(stackAt(nested, 110)).toEqual([
      { index: 0, top: -30, height: H },
      { index: 1, top: 10, height: H },
      { index: 2, top: 50, height: H },
    ]);
  });

  it("drops the held ancestors that the row at the line has left", () => {
    expect(stackAt(tree, 200)).toEqual([
      { index: 0, top: 0, height: H },
      { index: 1, top: H, height: H },
    ]);
  });

  it("pushes the group header up as the group's last row ends", () => {
    expect(stackAt(tree, 300)).toEqual([{ index: 0, top: -20, height: H }]);
  });

  it("lets the next group's header take over once it has scrolled past", () => {
    expect(stackAt(tree, 330)).toEqual([{ index: 8, top: 0, height: H }]);
  });

  it("measures the stack with the height each item has while held", () => {
    const items = outline("#first", "Root", "-Child", "--Leaf", "--Leaf2", "-Child2");
    const held = (item: Item) => (item.kind === "group" ? 30 : 20);
    const spans = spansOf(items, () => H);

    expect(stickyStack(items, spans, 100, held)).toEqual([
      { index: 0, top: 0, height: 30 },
      { index: 1, top: 30, height: 20 },
      { index: 2, top: 50, height: 20 },
    ]);
  });
});

describe("stackAbove", () => {
  const items = outline("#first", "Root", "-Child", "--Leaf", "#second", "Lone");
  const heightOf = (item: Item) => (item.kind === "group" ? 30 : 44);

  it.each([
    ["a group header", 0, 0],
    ["a top-level row", 1, 30],
    ["a nested row", 2, 30 + 44],
    ["a deeply nested row", 3, 30 + 44 + 44],
    ["a row in the next group", 5, 30],
  ])("is the room taken by the group header and ancestors of %s", (_, index, room) => {
    expect(stackAbove(items, index, heightOf)).toBe(room);
  });
});
