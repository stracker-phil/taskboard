import { beforeAll, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { registerCoreSettingsPages } from "../components/settings/pages";
import { hasSection, resolveSettingsPage, settingsGroups, settingsPages } from "./settingsPages";

beforeAll(() => {
  registerCoreSettingsPages();
});

describe("core settings pages", () => {
  it("lists the nav groups in the order Jira, Workspace, System", () => {
    expect(settingsGroups("").map((g) => g.group)).toEqual(["Jira", "Workspace", "System"]);
  });

  it.each([
    ["Jira", ["connection", "privacy", "projects", "sync"]],
    ["Workspace", ["spaces", "team", "statuses", "fields", "behavior"]],
    ["System", ["appearance", "shortcuts", "status", "about"]],
  ])("lists the %s pages in order", (group, ids) => {
    const entry = settingsGroups("").find((g) => g.group === group);

    expect(entry?.pages.map((p) => p.page.id)).toEqual(ids);
  });

  it("labels the Privacy & Data page and the Behavior page", () => {
    expect(resolveSettingsPage("privacy")?.label).toBe("Privacy & Data");
    expect(resolveSettingsPage("behavior")?.label).toBe("Behavior");
  });

  it("finds the Privacy & Data page by the title of its Jira data section", () => {
    const hits = settingsGroups("Jira data").flatMap((g) => g.pages);

    expect(hits.map((h) => [h.page.id, h.section?.id])).toEqual([["privacy", "cache"]]);
  });

  it("registers the Space groups as sections of the Spaces page", () => {
    const spaces = resolveSettingsPage("spaces")!;

    expect(spaces.sections?.map((s) => s.id)).toEqual(expect.arrayContaining(["layouts", "statuses", "fields"]));
    expect(spaces.sections?.find((s) => s.id === "fields")?.title).toBe("Custom fields");
  });
});

describe("resolveSettingsPage", () => {
  it("resolves a Space target to the Spaces page", () => {
    expect(resolveSettingsPage("space:10001")?.id).toBe("spaces");
  });

  it.each(["cache", "reads"])("resolves the %s section to the Privacy & Data page", (section) => {
    expect(resolveSettingsPage(section)?.id).toBe("privacy");
  });

  it("resolves the synced section to the Sync page", () => {
    expect(resolveSettingsPage("synced")?.id).toBe("sync");
  });

  it("resolves the meMode section to the Behavior page", () => {
    expect(resolveSettingsPage("meMode")?.id).toBe("behavior");
  });

  it("resolves the layouts section to the Spaces page", () => {
    expect(resolveSettingsPage("layouts")?.id).toBe("spaces");
  });

  it("resolves fields to the Custom fields page, not the Spaces page that also has a Custom fields section", () => {
    expect(resolveSettingsPage("fields")?.id).toBe("fields");
  });

  it("resolves statuses to the Statuses page, not the Spaces page that also has a Statuses section", () => {
    expect(resolveSettingsPage("statuses")?.id).toBe("statuses");
  });

  it("falls back to the first page for an unknown id", () => {
    expect(resolveSettingsPage("statuses-page")?.id).toBe(settingsPages()[0].id);
  });
});

describe("hasSection", () => {
  const spaces = () => resolveSettingsPage("spaces")!;

  it("is true on the Spaces page for any space:<projectId> target", () => {
    expect(hasSection(spaces(), "space:x")).toBe(true);
  });

  it("is false on another page for a space target", () => {
    expect(hasSection(resolveSettingsPage("sync")!, "space:x")).toBe(false);
  });

  it("is false on the Spaces page for an unknown section", () => {
    expect(hasSection(spaces(), "nope")).toBe(false);
  });
});
