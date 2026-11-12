import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type DiagnosticRow } from "../../api";
import { installCommands } from "../../commands";
import { createQueryClient } from "../../queries/queryClient";
import { findRow, resolveSettingsPage, settingsPages } from "../../registry/settingsPages";
import { resetStore } from "../../state/store";
import { connectionView } from "../../test/jira";
import { SettingsApp } from "../../windows/settings/SettingsApp";
import { registerCoreSettingsPages } from "./pages";

const mocked = vi.mocked(api, true);

const PAGE_IDS = ["connection", "privacy", "projects", "sync", "spaces", "team", "statuses", "fields", "behavior", "notifications", "appearance", "shortcuts", "status", "about"];

const diagnosticRow = (label: string): DiagnosticRow => ({ label, value: "Fine", tone: "info", command: null, commandLabel: null, copyText: null });
const diagnostics = () => ({ groups: ["Jira", "Sync", "This Mac", "Changes Sent to Jira"].map((title) => ({ title, rows: [diagnosticRow(`${title} detail`)] })) });

/** The elements the search can scroll to, as the page shows them. */
const anchored = () => [...document.querySelectorAll<HTMLElement>("[data-anchor]")];
const anchorIds = () => anchored().map((element) => element.dataset.anchor);

/** What the element shows as its title: a group's heading or a row's label. */
function shownTitle(element: HTMLElement): string | undefined {
  const text = element.matches(".settings-row") ? element.querySelector(".settings-row-text")?.firstElementChild : element.querySelector("h3");
  return text?.textContent?.trim();
}

async function openPage(id: string) {
  window.history.replaceState({}, "", `/?page=${id}`);
  render(<SettingsApp client={createQueryClient()} />);
  const page = resolveSettingsPage(id)!;
  const listed = (page.rows ?? []).filter((row) => !row.under).map((row) => row.id);
  await waitFor(() => expect(anchorIds()).toEqual(expect.arrayContaining(listed)));
  return page;
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  installCommands("settings");
  registerCoreSettingsPages();
  mocked.connectionGet.mockResolvedValue(connectionView({ tokenKind: "scoped", cloudId: "demo-cloud-1" }));
  mocked.diagnosticsGet.mockResolvedValue(diagnostics());
});

afterEach(() => {
  window.history.replaceState({}, "", "/");
});

describe("Settings search index", () => {
  it("covers every registered core page", () => {
    expect(settingsPages().map((page) => page.id)).toEqual(PAGE_IDS);
  });

  describe.each(PAGE_IDS)("the %s page", (id) => {
    it("only lists a fallback row that is another row of the page", () => {
      const rows = resolveSettingsPage(id)?.rows ?? [];

      for (const row of rows.filter((r) => r.under)) expect(rows.map((r) => r.id)).toContain(row.under);
    });

    it("shows every row it lists that has no fallback", async () => {
      const page = await openPage(id);

      const shown = anchorIds();
      for (const row of (page.rows ?? []).filter((r) => !r.under)) expect(shown).toContain(row.id);
    });

    it("lists every anchored element under the title it is shown with", async () => {
      const page = await openPage(id);

      for (const element of anchored()) {
        const entry = findRow(page, element.dataset.anchor);
        expect(entry, `an entry for ${element.dataset.anchor}`).toBeDefined();
        expect(entry?.title).toBe(shownTitle(element));
      }
    });
  });

  describe("Cloud ID", () => {
    it("is listed under the token kind", () => {
      expect(findRow(resolveSettingsPage("connection")!, "cloud-id")).toMatchObject({ title: "Cloud ID", under: "token-kind" });
    });

    it("shows once the token kind is scoped", async () => {
      mocked.connectionGet.mockResolvedValue(connectionView({ tokenKind: "classic", cloudId: null }));
      await openPage("connection");
      expect(anchorIds()).not.toContain("cloud-id");

      await userEvent.click(within(screen.getByRole("group", { name: "Token kind" })).getByRole("button", { name: /scoped/i }));

      expect(anchorIds()).toContain("cloud-id");
    });
  });
});
