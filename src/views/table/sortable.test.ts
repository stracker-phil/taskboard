import { describe, expect, it } from "vitest";
import type { SortBy } from "../../api";
import { nextSort, sortByOf } from "./sortable";

describe("sortByOf", () => {
  it.each(["key", "summary", "status", "priority", "due"])("lets the %s column sort by itself", (id) => {
    expect(sortByOf(id)).toBe(id);
  });

  it.each(["assignee", "epic", "labels", "customfield_10071"])("gives the %s column no sort", (id) => {
    expect(sortByOf(id)).toBeNull();
  });
});

describe("nextSort", () => {
  it.each<[SortBy, boolean]>([
    ["key", false],
    ["summary", false],
    ["status", false],
    ["priority", true],
    ["due", true],
  ])("starts a newly sorted %s column with descending=%s", (by, descending) => {
    expect(nextSort({ by: "updated", descending: false }, by)).toEqual({ by, descending });
  });

  it.each([true, false])("flips the direction of the sorted column (descending=%s)", (descending) => {
    expect(nextSort({ by: "key", descending }, "key")).toEqual({ by: "key", descending: !descending });
  });
});
