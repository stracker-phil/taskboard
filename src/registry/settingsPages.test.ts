import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { installCommands } from "../commands";
import { registerCoreSettingsPages } from "../components/settings/pages";
import { findRow, resolveSettingsPage, searchSettings, settingsGroups, settingsPages, targetOn } from "./settingsPages";

beforeEach(() => {
  installCommands("settings");
  registerCoreSettingsPages();
});

const page = (id: string) => resolveSettingsPage(id)!;
const hitsFor = (query: string) => searchSettings(query).map((hit) => (hit.row ? `${hit.page.id}/${hit.row.id}` : hit.page.id));

describe("core settings pages", () => {
  it("lists the nav groups in the order Jira, Workspace, System", () => {
    expect(settingsGroups().map((g) => g.group)).toEqual(["Jira", "Workspace", "System"]);
  });

  it.each([
    ["Jira", ["connection", "privacy", "projects", "sync"]],
    ["Workspace", ["spaces", "team", "statuses", "fields", "behavior", "notifications"]],
    ["System", ["appearance", "shortcuts", "status", "about"]],
  ])("lists the %s pages in order", (group, ids) => {
    const entry = settingsGroups().find((g) => g.group === group);

    expect(entry?.pages.map((p) => p.id)).toEqual(ids);
  });

  it("labels the Privacy & Data page and the Behavior page", () => {
    expect(page("privacy").label).toBe("Privacy & Data");
    expect(page("behavior").label).toBe("Behavior");
  });

  it("registers a row for every installed command on the Shortcuts page", () => {
    expect(page("shortcuts").rows?.some((row) => row.id.startsWith("shortcut:"))).toBe(true);
  });
});

describe("resolveSettingsPage", () => {
  it("resolves a page id to that page", () => {
    expect(resolveSettingsPage("sync")?.id).toBe("sync");
  });

  it.each([
    ["cache", "privacy"],
    ["cloud-id", "connection"],
    ["synced", "sync"],
    ["meMode", "behavior"],
    ["mode", "appearance"],
    ["about-logs", "about"],
  ])("resolves the row %s to the %s page that holds it", (row, expected) => {
    expect(resolveSettingsPage(row)?.id).toBe(expected);
  });

  it("resolves a page id before a row of another page", () => {
    expect(resolveSettingsPage("statuses")?.id).toBe("statuses");
    expect(resolveSettingsPage("fields")?.id).toBe("fields");
  });

  it("resolves a space:<projectId> target to the Spaces page", () => {
    expect(resolveSettingsPage("space:10001")?.id).toBe("spaces");
  });

  it.each([["no-such-id"], [undefined]])("falls back to the first page for %s", (id) => {
    expect(resolveSettingsPage(id)?.id).toBe(settingsPages()[0].id);
  });
});

describe("targetOn", () => {
  it("names a row with the row it is revealed under", () => {
    expect(targetOn(page("connection"), "cloud-id")).toEqual({ id: "cloud-id", under: "token-kind" });
  });

  it("names a row that has no fallback by its id alone", () => {
    expect(targetOn(page("connection"), "site")).toEqual({ id: "site" });
  });

  it("names a Space on the Spaces page", () => {
    expect(targetOn(page("spaces"), "space:10001")).toEqual({ id: "space:10001" });
  });

  it.each([
    ["sync", "space:10001"],
    ["connection", "connection"],
    ["connection", "no-such-row"],
    ["spaces", undefined],
  ])("names nothing on the %s page for %s", (id, target) => {
    expect(targetOn(page(id), target)).toBeUndefined();
  });
});

describe("findRow", () => {
  it("finds a row of the page by id", () => {
    expect(findRow(page("connection"), "cloud-id")?.title).toBe("Cloud ID");
  });

  it("finds nothing for an id the page does not list", () => {
    expect(findRow(page("connection"), "cache")).toBeUndefined();
  });
});

describe("searchSettings", () => {
  it("finds a row by its title with its page", () => {
    const [first] = searchSettings("Cloud ID");

    expect(first.page.id).toBe("connection");
    expect(first.row).toMatchObject({ id: "cloud-id", title: "Cloud ID" });
  });

  it("finds a row by one of its keywords", () => {
    expect(hitsFor("keychain")).toContain("connection/api-token");
  });

  it("ranks a row whose title starts with the text above rows that only list it as a keyword", () => {
    const hits = hitsFor("token");

    expect(hits.indexOf("connection/token-kind")).toBeLessThan(hits.indexOf("connection/token-expires"));
    expect(hits.indexOf("connection/api-token")).toBeLessThan(hits.indexOf("connection/token-expires"));
  });

  it("finds a page by its label before its rows", () => {
    expect(hitsFor("appearance")[0]).toBe("appearance");
  });

  it("finds a page by one of its keywords, without a row", () => {
    expect(searchSettings("keyboard").some((hit) => hit.page.id === "shortcuts" && !hit.row)).toBe(true);
  });

  it("ignores case and surrounding blanks", () => {
    expect(hitsFor("  CLOUD id ")[0]).toBe("connection/cloud-id");
  });

  it("finds nothing for text that matches no page or row", () => {
    expect(searchSettings("zzzzqq")).toEqual([]);
  });
});
