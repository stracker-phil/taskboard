import { describe, expect, it } from "vitest";
import { LIST_ICONS, listIcon } from "./listIcons";

describe("listIcon", () => {
  it("returns the icon stored under a known name", () => {
    expect(listIcon("rocket")).toBe(LIST_ICONS.rocket);
    expect(listIcon("rocket")).not.toBe(LIST_ICONS.funnel);
  });

  it.each([[null], [undefined], [""], ["no-such-icon"]])("falls back to the funnel for %j", (name) => {
    expect(listIcon(name)).toBe(LIST_ICONS.funnel);
  });
});
