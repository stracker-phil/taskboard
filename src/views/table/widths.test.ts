import { describe, expect, it } from "vitest";
import { columnWidth } from "./widths";

describe("columnWidth", () => {
  it("uses the width the owner dragged the column to", () => {
    expect(columnWidth("key", { key: 130 })).toBe(130);
  });

  it.each([
    ["key", 102],
    ["status", 130],
    ["priority", 104],
    ["customfield_10071", 90],
  ])("falls back to the default of %s", (id, width) => {
    expect(columnWidth(id, {})).toBe(width);
  });

  it("ignores the widths of other columns", () => {
    expect(columnWidth("status", { key: 200 })).toBe(130);
  });
});
