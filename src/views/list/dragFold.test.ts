import { describe, expect, it } from "vitest";
import { groupOf, row, viewData } from "../../test/issues";
import { FOLD_ROWS, foldItems, foldLayout, foldSpans, foldStart } from "./dragFold";
import { flatten, type Item } from "./flatten";

const ROW_PX = 44;
const GROUP_PX = 30;
const MORE_PX = 20;

const rows = (...ids: string[]) => ids.map((id) => row({ id, key: `NW-${id}`, summary: `Issue ${id}` }));

function itemsOf(groups: [string, string[], boolean?][], groupBy: "status" | "none" = "status"): Item[] {
  const all = groups.flatMap(([, ids]) => rows(...ids));
  const data = viewData(
    groups.map(([key, ids]) => groupOf(rows(...ids), { key, label: key })),
    all,
    { groupBy },
  );
  const closed = new Set(groups.filter(([, , collapsed]) => collapsed).map(([key]) => key));
  return flatten(data, (group) => !closed.has(group.key));
}

/** Folded, with every group open as `foldItems` expects; group "c" is closed in the normal view. */
function scene() {
  const items = foldItems(itemsOf([["a", ["1", "2", "3"]], ["b", ["4"]], ["c", ["5", "6"]]]));
  const full = (index: number) => (items[index].kind === "row" ? ROW_PX : items[index].kind === "group" ? GROUP_PX : 0);
  const normal = (index: number) => {
    const item = items[index];
    if (item.kind === "more") return 0;
    if (item.kind === "row" && item.groupKey === "c") return 0;
    return full(index);
  };
  const at = (id: string) => items.findIndex((item) => item.kind === "row" && item.row.id === id);
  const moreOf = (key: string) => items.findIndex((item) => item.kind === "more" && item.groupKey === key);
  const layout = (progress: number, dragged = "1") => foldLayout(items, progress, dragged, full, normal, MORE_PX);
  return { items, at, moreOf, layout, full };
}

describe("foldItems", () => {
  it("adds one more piece after each group's rows", () => {
    const items = foldItems(itemsOf([["a", ["1", "2"]], ["b", ["3"]]]));

    expect(items.map((item) => item.kind)).toEqual(["group", "row", "row", "more", "group", "row", "more"]);
    expect(items.filter((item) => item.kind === "more").map((item) => (item as { groupKey: string }).groupKey)).toEqual(["a", "b"]);
  });

  it("counts the rows beyond the rows a folded group keeps as hidden", () => {
    const count = FOLD_ROWS + 3;
    const ids = Array.from({ length: count }, (_, i) => String(i + 1));
    const items = foldItems(itemsOf([["a", ids], ["b", ["x"]]]));

    const more = items.filter((item): item is Extract<Item, { kind: "more" }> => item.kind === "more");
    expect(more.map((item) => item.hidden)).toEqual([3, 0]);
  });

  it("closes a group without rows with a piece that hides nothing", () => {
    const items = foldItems(itemsOf([["a", []]]));

    expect(items).toEqual([expect.objectContaining({ kind: "group" }), { kind: "more", groupKey: "a", tab: true, hidden: 0 }]);
  });

  it.each([
    ["a grouped view", "status" as const, true],
    ["an ungrouped view", "none" as const, false],
  ])("takes the tab flag from the header of %s", (_name, groupBy, tab) => {
    const items = foldItems(itemsOf([["a", ["1"]]], groupBy));

    expect(items.find((item) => item.kind === "more")).toMatchObject({ tab });
  });
});

describe("foldLayout", () => {
  describe("at progress 0", () => {
    it("shows the normal heights, a closed group's rows and every more piece at 0", () => {
      const { items, layout, at, moreOf } = scene();

      const { heights } = layout(0);

      expect(heights[at("1")]).toBe(ROW_PX);
      expect(heights[at("3")]).toBe(ROW_PX);
      expect(heights[at("5")]).toBe(0);
      expect(heights[moreOf("a")]).toBe(0);
      expect(heights).toHaveLength(items.length);
    });

    it("shows only the groups that are open in the normal view as open", () => {
      const { layout } = scene();

      expect([...layout(0).open].sort()).toEqual(["a", "b"]);
    });

    it("gives no row the corners of a closed card yet", () => {
      expect(scene().layout(0).positions.size).toBe(0);
    });
  });

  describe("at progress 1", () => {
    it("gives every group the same total height of the kept rows and the more piece", () => {
      const { items, layout } = scene();

      const { heights } = layout(1);

      const totals: number[] = [];
      let total = 0;
      items.forEach((item, index) => {
        if (item.kind === "group") total = 0;
        else total += heights[index];
        if (item.kind === "more") totals.push(total);
      });
      expect(totals).toEqual([FOLD_ROWS * ROW_PX + MORE_PX, FOLD_ROWS * ROW_PX + MORE_PX, FOLD_ROWS * ROW_PX + MORE_PX]);
    });

    it("keeps the first rows of a group and folds the rest to 0", () => {
      const { layout, at } = scene();

      const { heights } = layout(1);

      expect(heights[at("1")]).toBe(ROW_PX);
      expect(heights[at("2")]).toBe(ROW_PX);
      expect(heights[at("3")]).toBe(0);
    });

    it("lets the dragged row replace the last kept row when it would fold away", () => {
      const { layout, at } = scene();

      const { heights } = layout(1, "3");

      expect(heights[at("1")]).toBe(ROW_PX);
      expect(heights[at("2")]).toBe(0);
      expect(heights[at("3")]).toBe(ROW_PX);
    });

    it("fills the more piece of a group with fewer rows up to the common height", () => {
      const { layout, moreOf } = scene();

      const { heights } = layout(1);

      expect(heights[moreOf("b")]).toBe(ROW_PX + MORE_PX);
      expect(heights[moreOf("a")]).toBe(MORE_PX);
    });

    it("opens every group, a closed one too", () => {
      expect([...scene().layout(1).open].sort()).toEqual(["a", "b", "c"]);
    });

    it("gives the rows of every card but the last piece the middle or first corners", () => {
      const { layout, at } = scene();

      const { positions } = layout(1);

      expect(positions.get(at("1"))).toBe("first");
      expect(positions.get(at("2"))).toBe("middle");
      expect(positions.get(at("4"))).toBe("first");
      expect(positions.get(at("5"))).toBe("first");
    });
  });

  describe("in between", () => {
    it("moves heights linearly from the normal to the folded height", () => {
      const { layout, at, moreOf } = scene();

      const { heights } = layout(0.5);

      expect(heights[at("3")]).toBe(ROW_PX / 2);
      expect(heights[at("5")]).toBe(ROW_PX / 2);
      expect(heights[moreOf("a")]).toBe(MORE_PX / 2);
    });

    it("fades the rows that fold away out and the rows of a closed group in", () => {
      const { layout, at } = scene();

      const { opacity } = layout(0.25);

      expect(opacity[at("3")]).toBeCloseTo(0.75);
      expect(opacity[at("5")]).toBeCloseTo(0.25);
    });

    it("keeps kept rows and the other items fully visible", () => {
      const { items, layout, at } = scene();

      const { opacity } = layout(0.25);

      expect(opacity[at("1")]).toBe(1);
      expect(opacity[at("4")]).toBe(1);
      const others = items.map((item, index) => (item.kind === "row" ? 1 : opacity[index]));
      expect(others.every((value) => value === 1)).toBe(true);
    });

    it("rounds a card only once its more piece is 10px tall", () => {
      const { layout, at } = scene();

      const { positions } = layout(0.4);

      expect(positions.has(at("1"))).toBe(false);
      expect(positions.get(at("4"))).toBe("first");
    });

    it("lists as open the groups that have a row with height", () => {
      const { layout } = scene();

      expect([...layout(0.1).open].sort()).toEqual(["a", "b", "c"]);
    });
  });
});

describe("foldSpans", () => {
  it("spans each group from its first row to the end of its more piece, below the header", () => {
    const { layout, items } = scene();
    const { heights } = layout(1);
    const card = FOLD_ROWS * ROW_PX + MORE_PX;

    const spans = foldSpans(items, heights, 5);

    expect(spans.get("a")).toEqual({ top: 5 + GROUP_PX, height: card, tab: true });
    expect(spans.get("b")).toEqual({ top: 5 + GROUP_PX + card + GROUP_PX, height: card, tab: true });
    expect(spans.get("c")).toMatchObject({ top: 5 + 2 * (GROUP_PX + card) + GROUP_PX, height: card });
  });

  it("starts a group without rows at its more piece", () => {
    const items = foldItems(itemsOf([["a", []]]));

    const spans = foldSpans(items, [GROUP_PX, 30], 0);

    expect(spans.get("a")).toEqual({ top: GROUP_PX, height: 30, tab: true });
  });
});

describe("foldStart", () => {
  it("adds the heights before the item to the start padding", () => {
    expect(foldStart([10, 20, 30], 2, 5)).toBe(35);
  });

  it("is the padding for the first item", () => {
    expect(foldStart([10, 20], 0, 5)).toBe(5);
  });
});
