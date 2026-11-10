import { beforeAll, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { registerCoreSettingsPages } from "../components/settings/pages";
import { hasSection, resolveSettingsPage, settingsPages } from "./settingsPages";

beforeAll(() => {
  registerCoreSettingsPages();
});

describe("core settings pages", () => {
  it("has an app-wide Statuses page in the Jira group after Team", () => {
    const pages = settingsPages();
    const ids = pages.map((p) => p.id);

    expect(pages.find((p) => p.id === "statuses")?.group).toBe("Jira");
    expect(ids.indexOf("statuses")).toBe(ids.indexOf("team") + 1);
  });

  it("has an app-wide Custom fields page in the Jira group between Statuses and Sync", () => {
    const pages = settingsPages();
    const ids = pages.map((p) => p.id);

    expect(pages.find((p) => p.id === "fields")?.label).toBe("Custom fields");
    expect(pages.find((p) => p.id === "fields")?.group).toBe("Jira");
    expect(ids.indexOf("fields")).toBe(ids.indexOf("statuses") + 1);
    expect(ids.indexOf("sync")).toBe(ids.indexOf("fields") + 1);
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
