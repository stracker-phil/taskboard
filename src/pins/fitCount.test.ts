import { describe, expect, it } from "vitest";
import { fitCount } from "./fitCount";

describe("fitCount", () => {
  it.each([
    ["all items when there is room", 500, [100, 100, 100], 2, 3],
    ["all items when they fit exactly, gaps included", 304, [100, 100, 100], 2, 3],
    ["the items before the first that overflows", 250, [100, 100, 100], 2, 2],
    ["none when the first is wider than the row", 80, [100, 50], 2, 0],
    ["none when there are no items", 300, [], 2, 0],
    ["the first item alone when the second overflows only with the gap", 124, [60, 61], 4, 1],
  ])("counts %s", (_name, available, widths, gap, expected) => {
    expect(fitCount(available, widths, gap)).toBe(expected);
  });

  it("counts an unmeasured item as taking no room", () => {
    expect(fitCount(100, [0, 0, 90], 0)).toBe(3);
  });

  it("leaves out an item that fits only without the gap before it", () => {
    expect(fitCount(200, [100, 100], 2)).toBe(1);
  });
});
