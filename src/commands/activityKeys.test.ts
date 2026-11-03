import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { installCommands } from ".";
import { resetStore, useStore } from "../state/store";
import { allCommands, clearCommands, resolveKeydown, setOverrides } from "./registry";

const SCOPES = ["global", "main"];

/** ⌥ plus a digit the way macOS reports it: the composed character in `key`, the physical key in `code`. */
function altDigit(n: number, composed: string) {
  return resolveKeydown({ key: composed, code: `Digit${n}`, metaKey: false, shiftKey: false, ctrlKey: false, altKey: true }, SCOPES);
}

const openIssue = () => useStore.getState().openIssue("10042", { listId: 11, viewId: 101 });

beforeEach(() => {
  installCommands("main");
  setOverrides({});
  resetStore();
});

afterEach(() => {
  clearCommands();
});

describe("activity keys", () => {
  it("registers activity.all, activity.comments and activity.changes on ⌥1 to ⌥3", () => {
    const filters = allCommands().filter((c) => c.id.startsWith("activity."));

    expect(filters.map((c) => [c.id, c.defaultBinding])).toEqual([
      ["activity.all", "Alt+1"],
      ["activity.comments", "Alt+2"],
      ["activity.changes", "Alt+3"],
    ]);
  });

  it.each([
    [1, "¡", "all"],
    [2, "™", "comments"],
    [3, "£", "changes"],
  ])("⌥%i chooses the %s filter while the Issue view is open", async (n, composed, filter) => {
    openIssue();
    useStore.setState({ activityFilter: filter === "all" ? "changes" : "all" });

    await altDigit(n, composed)?.run();

    expect(useStore.getState().activityFilter).toBe(filter);
  });

  it.each([
    [1, "¡"],
    [2, "™"],
    [3, "£"],
  ])("⌥%i does nothing when no Issue view is open", (n, composed) => {
    expect(altDigit(n, composed)).toBeNull();
    expect(useStore.getState().activityFilter).toBe("all");
  });

  it("follows a rebound key and frees the old one", async () => {
    openIssue();
    setOverrides({ "activity.changes": "Alt+9" });

    expect(altDigit(3, "£")).toBeNull();
    const command = resolveKeydown({ key: "ª", code: "Digit9", metaKey: false, shiftKey: false, ctrlKey: false, altKey: true }, SCOPES);
    await command?.run();

    expect(useStore.getState().activityFilter).toBe("changes");
  });
});
