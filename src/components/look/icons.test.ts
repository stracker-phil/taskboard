import { describe, expect, it } from "vitest";
import { DEFAULT_LIST_ICON, LIST_ICONS, lineIcon, listIcon } from "./icons";

const NEW_ICONS = [
  "eye", "git-pull-request", "repeat", "kanban", "milestone", "mountain", "archive", "tag", "hourglass", "ban", "siren",
  "shield-alert", "key", "test-tube", "gauge", "accessibility", "store", "shopping-bag", "receipt", "credit-card", "percent",
  "building", "life-buoy", "plug", "puzzle", "blocks", "paintbrush", "languages", "book-open", "database", "server",
  "webhook", "smartphone", "chart-bar",
];

describe("listIcon", () => {
  it("returns the icon stored under a known name", () => {
    expect(listIcon("rocket")).toBe(LIST_ICONS.rocket);
    expect(listIcon("rocket")).not.toBe(LIST_ICONS.folder);
  });

  it.each([[null], [undefined], [""], ["no-such-icon"]])("falls back to the folder for %j", (name) => {
    expect(listIcon(name)).toBe(LIST_ICONS.folder);
    expect(DEFAULT_LIST_ICON).toBe(LIST_ICONS.folder);
  });

  it("still lets the funnel be picked by name", () => {
    expect(listIcon("funnel")).toBe(LIST_ICONS.funnel);
    expect(listIcon("funnel")).not.toBe(LIST_ICONS.folder);
  });

  it.each(NEW_ICONS)("knows the %s icon", (name) => {
    expect(LIST_ICONS[name]).toBeDefined();
    expect(listIcon(name)).toBe(LIST_ICONS[name]);
    expect(listIcon(name)).not.toBe(DEFAULT_LIST_ICON);
  });
});

describe("lineIcon", () => {
  it("returns the icon for a known name and null for a letter or an emoji", () => {
    expect(lineIcon("kanban")).toBe(LIST_ICONS.kanban);
    expect(lineIcon("A")).toBeNull();
    expect(lineIcon(null)).toBeNull();
  });
});
