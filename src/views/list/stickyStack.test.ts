import { describe, expect, it } from "vitest";
import { outline, spansOf } from "../../test/items";
import type { Item } from "./flatten";
import { stackAbove, stickyTracks } from "./stickyStack";

const H = 40;
const evenly = () => H;

/** Everything is 40 high, so item `i` lies between `40 * i` and `40 * (i + 1)`. */
const tracksAt = (items: Item[], drawn: number[], scrollTop = 0, heightAt: (index: number) => number = evenly) =>
  stickyTracks(items, spansOf(items, evenly), drawn, scrollTop, heightAt);

const all = (items: Item[]) => items.map((_, index) => index);

describe("stickyTracks", () => {
  const tree = outline("#first", "Root", "-Child", "--Leaf", "--Leaf2", "-Child2", "-Child3", "Root2", "#second", "Lone");

  it("gives a track to each group and each parent row, deepest first and groups last", () => {
    expect(tracksAt(tree, all(tree))).toEqual([
      { index: 2, top: 2 * H, height: 3 * H, offset: 2 * H, depth: 1 },
      { index: 1, top: H, height: 6 * H, offset: H, depth: 0 },
      { index: 0, top: 0, height: 8 * H, offset: 0, depth: -1 },
      { index: 8, top: 8 * H, height: 2 * H, offset: 0, depth: -1 },
    ]);
  });

  it("gives leaves no track", () => {
    const leaves = outline("#first", "A", "B", "C");

    expect(tracksAt(leaves, [1, 2, 3]).map((track) => track.index)).toEqual([0]);
  });

  it("adds the group and the ancestors of a drawn deep row although they are not drawn", () => {
    expect(tracksAt(tree, [3]).map((track) => track.index)).toEqual([2, 1, 0]);
  });

  it("tracks only the groups and parents of what is drawn", () => {
    expect(tracksAt(tree, [9]).map((track) => track.index)).toEqual([8]);
  });

  it("leaves out a track whose end has scrolled above where it would stick", () => {
    expect(tracksAt(tree, all(tree), 5 * H).map((track) => track.index)).toEqual([1, 0, 8]);
    expect(tracksAt(tree, all(tree), 7 * H).map((track) => track.index)).toEqual([0, 8]);
  });

  it("keeps a track whose end lies below its stick line", () => {
    const [child] = tracksAt(tree, all(tree), 100);

    expect(child).toMatchObject({ index: 2, offset: 2 * H });
  });

  it("starts a group's track where its header ends, since its span may begin with a gap", () => {
    const items = outline("#first", "A", "#second", "B");
    const spans = spansOf(items, (item) => (item.kind === "group" ? 50 : H));

    const tracks = stickyTracks(items, spans, [2], 0, (index) => (items[index].kind === "group" ? 30 : H));

    expect(tracks).toEqual([{ index: 2, top: spans[2].end - 30, height: spans[3].end - (spans[2].end - 30), offset: 0, depth: -1 }]);
  });

  it("sticks nested tracks below the group header and ancestors, measured by their heights", () => {
    const items = outline("#first", "Root", "-Child", "--Leaf");
    const heightAt = (index: number) => (index === 0 ? 30 : 20);

    expect(tracksAt(items, [3], 0, heightAt).map(({ index, offset }) => [index, offset])).toEqual([
      [2, 50],
      [1, 30],
      [0, 0],
    ]);
  });
});

describe("stackAbove", () => {
  const items = outline("#first", "Root", "-Child", "--Leaf", "#second", "Lone");
  const heightAt = (index: number) => (items[index].kind === "group" ? 30 : 44);

  it.each([
    ["a group header", 0, 0],
    ["a top-level row", 1, 30],
    ["a nested row", 2, 30 + 44],
    ["a deeply nested row", 3, 30 + 44 + 44],
    ["a row in the next group", 5, 30],
  ])("is the room taken by the group header and ancestors of %s", (_, index, room) => {
    expect(stackAbove(items, index, heightAt)).toBe(room);
  });
});
