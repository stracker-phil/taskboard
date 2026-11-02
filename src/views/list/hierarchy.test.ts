import { describe, expect, it } from "vitest";
import { outline } from "../../test/items";
import { ancestorAt, subtreeEnd, treeGuides } from "./hierarchy";

const guides = (items: ReturnType<typeof outline>, target: string | null) => Object.fromEntries(treeGuides(items, target));

describe("treeGuides", () => {
  const items = outline("#a", "Root", "-Early", "--EarlyChild", "-Parent", "--Before", "--Target", "--After", "-Later", "--LaterChild", "NextRoot");

  it("shapes each level: a branch for a child with a later sibling, a curve for the last child, a line passing by or nothing below", () => {
    expect(guides(items, null)).toEqual({
      Early: "+-",
      EarlyChild: "|-L-",
      Parent: "+-",
      Before: "|-+-",
      Target: "|-+-",
      After: "|-L-",
      Later: "L-",
      LaterChild: " -L-",
    });
  });

  it("does not treat rows on either side of a group boundary as siblings", () => {
    const grouped = outline("#a", "Root", "-Early", "#b", "-Orphan", "--Child");

    expect(guides(grouped, null)).toEqual({ Early: "L-", Orphan: "L-", Child: " -L-" });
  });

  it("lights the path from the target up to its top-level ancestor, level by level", () => {
    expect(guides(items, "Target")).toEqual({
      Early: "+p",
      EarlyChild: "|pL-",
      Parent: "+p",
      Before: "|-+p",
      Target: "|-+e",
      After: "|-L-",
      Later: "L-",
      LaterChild: " -L-",
    });
  });

  it("marks exactly one level as the end of the path: the target's last level", () => {
    const marks = guides(items, "Target");
    const ends = Object.entries(marks).flatMap(([id, row]) => [...row.matchAll(/.e/g)].map((end) => [id, end.index]));

    expect(ends).toEqual([["Target", 2]]);
  });

  it("ends the path at a depth-1 target and leaves its subtree unlit", () => {
    expect(guides(items, "Parent")).toEqual({
      Early: "+p",
      EarlyChild: "|pL-",
      Parent: "+e",
      Before: "|-+-",
      Target: "|-+-",
      After: "|-L-",
      Later: "L-",
      LaterChild: " -L-",
    });
  });

  it.each([
    ["no target", null],
    ["a top-level target", "Root"],
    ["a target that is not shown", "Missing"],
  ])("lights nothing for %s", (_, target) => {
    expect(Object.values(guides(items, target)).every((marks) => /^(.-)+$/.test(marks))).toBe(true);
  });

  it("gives every nested row two characters per level and none to top-level rows", () => {
    const marks = guides(items, null);

    expect(marks).toMatchObject({ Early: "+-", EarlyChild: "|-L-", Target: "|-+-" });
    expect(marks).not.toHaveProperty("Root");
    expect(marks).not.toHaveProperty("NextRoot");
  });

  it("stops the path at the start of its group", () => {
    const grouped = outline("#a", "Root", "-Early", "#b", "-Orphan", "--Target");

    expect(guides(grouped, "Target")).toEqual({ Early: "L-", Orphan: "L-", Target: " -Le" });
  });
});

describe("subtreeEnd", () => {
  const items = outline("#a", "Root", "-Child", "--Grandchild", "-Other", "NextRoot", "#b", "Lone", "-Nested");

  it.each([
    ["a top-level row", 1, 4],
    ["a mid-level row", 2, 3],
    ["a row with nothing nested under it", 4, 4],
    ["the last row before a group", 5, 5],
  ])("is the last item under %s", (_, index, end) => {
    expect(subtreeEnd(items, index)).toBe(end);
  });

  it("does not run into the next group", () => {
    expect(subtreeEnd(items, 7)).toBe(8);
    expect(subtreeEnd(outline("#a", "Root", "-Child", "#b", "-Other"), 1)).toBe(2);
  });
});

describe("ancestorAt", () => {
  const items = outline("#a", "Root", "-Child", "--Grandchild", "-Other", "NextRoot", "-NextChild");

  it.each([
    ["the parent", 3, 1, 2],
    ["the top-level ancestor", 3, 0, 1],
    ["the nearest top-level row of a later subtree", 6, 0, 5],
  ])("finds %s", (_, from, depth, found) => {
    expect(ancestorAt(items, from, depth)).toBe(found);
  });

  it("is -1 when a shallower row comes before any row at that depth", () => {
    expect(ancestorAt(outline("#a", "Root", "-Child", "NextRoot", "-NextChild"), 4, 1)).toBe(-1);
  });

  it("is -1 when the group starts first", () => {
    expect(ancestorAt(outline("#a", "Root", "#b", "-Orphan"), 3, 0)).toBe(-1);
  });
});
