import { beforeAll, describe, expect, it, vi } from "vitest";

vi.mock("../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../test/apiMock")).createApiMock() }));

import { registerCoreSettingsPages } from "../components/settings/pages";
import { hasSection, resolveSettingsPage, settingsPages } from "./settingsPages";

beforeAll(() => {
  registerCoreSettingsPages();
});

describe("core settings pages", () => {
  it("has no app-wide Statuses page, statuses being per Space", () => {
    expect(settingsPages().map((p) => p.id)).not.toContain("statuses");
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

  it.each([["layouts"], ["statuses"], ["fields"]])("resolves the %s section to the Spaces page", (section) => {
    expect(resolveSettingsPage(section)?.id).toBe("spaces");
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
