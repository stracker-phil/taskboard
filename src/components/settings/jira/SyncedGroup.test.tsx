import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { formatLocalTime } from "../../../lib/time";
import { createQueryClient } from "../../../queries/queryClient";
import { SYNC_KEY } from "../../../queries/sync";
import { resetStore, useStore } from "../../../state/store";
import { projectCounts, syncOverview } from "../../../test/jira";
import { defaultSync, settings } from "../../../test/settings";
import { CacheGroup } from "./CacheGroup";

const mocked = vi.mocked(api, true);

function renderGroup() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <CacheGroup />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.syncOverview.mockResolvedValue(syncOverview());
  mocked.getSettings.mockResolvedValue(settings());
});

describe("CacheGroup", () => {
  describe("sync state", () => {
    it("says the cache was never synced", async () => {
      renderGroup();

      expect(await screen.findByText("Not synced yet")).toBeInTheDocument();
    });

    it("names the time of the last successful sync", async () => {
      const lastOk = "2026-03-04T09:30:00Z";
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: { state: "idle", lastOk } }));

      renderGroup();

      expect(await screen.findByText(`Synced at ${formatLocalTime(lastOk)}`)).toBeInTheDocument();
    });

    it("keeps Sync Now disabled until the overview has loaded", async () => {
      renderGroup();
      expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled();

      await screen.findByText("Not synced yet");

      expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled();
    });

    it("queues a sync from Sync Now", async () => {
      renderGroup();
      await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled());

      await userEvent.click(screen.getByRole("button", { name: "Sync Now" }));

      expect(mocked.syncRun).toHaveBeenCalled();
    });

    it("disables Sync Now while a sync runs", async () => {
      mocked.syncOverview.mockResolvedValue(syncOverview({ state: { state: "running", job: "full" } }));

      renderGroup();

      expect(await screen.findByText("Syncing…")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled();
    });
  });

  describe("clearing the cache on quit", () => {
    const switchName = "Clear the cache when Taskboard quits";

    it("shows the switch off when the setting is off", async () => {
      renderGroup();

      expect(await screen.findByRole("switch", { name: switchName })).not.toBeChecked();
    });

    it("shows the switch on when the setting is saved on", async () => {
      mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), clearCacheOnQuit: true } }));

      renderGroup();

      expect(await screen.findByRole("switch", { name: switchName })).toBeChecked();
    });

    it("saves the whole sync settings with the flag turned on and shows the saved result", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("switch", { name: switchName }));

      await waitFor(() => expect(mocked.syncSave).toHaveBeenCalledWith({ ...defaultSync(), clearCacheOnQuit: true }));
      await waitFor(() => expect(screen.getByRole("switch", { name: switchName })).toBeChecked());
    });

    it("saves the flag turned off when it was on", async () => {
      mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), keepDoneDays: 30, clearCacheOnQuit: true } }));
      renderGroup();

      await userEvent.click(await screen.findByRole("switch", { name: switchName }));

      await waitFor(() => expect(mocked.syncSave).toHaveBeenCalledWith({ ...defaultSync(), keepDoneDays: 30, clearCacheOnQuit: false }));
      await waitFor(() => expect(screen.getByRole("switch", { name: switchName })).not.toBeChecked());
    });

    it("keeps the switch as saved when saving fails", async () => {
      mocked.syncSave.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderGroup();

      await userEvent.click(await screen.findByRole("switch", { name: switchName }));

      await waitFor(() => expect(mocked.syncSave).toHaveBeenCalled());
      expect(screen.getByRole("switch", { name: switchName })).not.toBeChecked();
    });
  });

  describe("keeping Jira data in memory only", () => {
    const switchName = "Keep Jira data in memory only";
    const clearOnQuit = "Clear the cache when Taskboard quits";
    const toasts = () => useStore.getState().toasts.map((t) => t.message);

    it("describes what the switch does", async () => {
      renderGroup();

      await screen.findByRole("switch", { name: switchName });

      expect(screen.getByText("Nothing from Jira is written to disk. Each launch syncs again.")).toBeInTheDocument();
    });

    it("shows the switch off when the setting is off", async () => {
      renderGroup();

      expect(await screen.findByRole("switch", { name: switchName })).not.toBeChecked();
    });

    it("shows the switch on when the setting is saved on", async () => {
      mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), memoryOnly: true } }));

      renderGroup();

      expect(await screen.findByRole("switch", { name: switchName })).toBeChecked();
    });

    it("saves the whole sync settings with memory-only turned on and tells that the cache is empty", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("switch", { name: switchName }));

      await waitFor(() => expect(mocked.syncSave).toHaveBeenCalledWith({ ...defaultSync(), memoryOnly: true }));
      await waitFor(() => expect(screen.getByRole("switch", { name: switchName })).toBeChecked());
      expect(toasts()).toContain("The cache is empty; Taskboard fetches your issues again.");
    });

    it("shows no success toast and keeps the switch as saved when saving fails", async () => {
      mocked.syncSave.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderGroup();

      await userEvent.click(await screen.findByRole("switch", { name: switchName }));

      await waitFor(() => expect(mocked.syncSave).toHaveBeenCalled());
      await waitFor(() => expect(toasts()).toContain("Could not save the setting: Disk full"));
      expect(toasts()).not.toContain("The cache is empty; Taskboard fetches your issues again.");
      expect(screen.getByRole("switch", { name: switchName })).not.toBeChecked();
    });

    it("offers clearing the cache on quit while memory-only is off", async () => {
      renderGroup();

      expect(await screen.findByRole("switch", { name: clearOnQuit })).toBeInTheDocument();
    });

    it("hides clearing the cache on quit while memory-only is on", async () => {
      mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), memoryOnly: true } }));

      renderGroup();

      await screen.findByRole("switch", { name: switchName });
      expect(screen.queryByRole("switch", { name: clearOnQuit })).not.toBeInTheDocument();
    });
  });

  describe("keeping read history", () => {
    const groupName = "Keep history you've read";
    const pressed = () =>
      within(screen.getByRole("group", { name: groupName }))
        .getAllByRole("button")
        .filter((b) => b.getAttribute("aria-pressed") === "true")
        .map((b) => b.textContent);

    it("presses the option matching the saved number of days", async () => {
      renderGroup();
      await screen.findByRole("group", { name: groupName });

      expect(pressed()).toEqual(["90 days"]);
    });

    it("presses none when the saved number of days is not an option", async () => {
      mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), keepHistoryDays: 45 } }));
      renderGroup();
      await screen.findByRole("group", { name: groupName });

      expect(pressed()).toEqual([]);
    });

    it("saves the whole sync settings with the chosen days and presses the saved option", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("button", { name: "180 days" }));

      await waitFor(() => expect(mocked.syncSave).toHaveBeenCalledWith({ ...defaultSync(), keepHistoryDays: 180 }));
      await waitFor(() => expect(pressed()).toEqual(["180 days"]));
    });
  });

  describe("reset confirmation", () => {
    it("says that what you've read stays", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("button", { name: "Reset Cache" }));

      expect(screen.getByRole("alertdialog")).toHaveTextContent("bookmarks and what you've read stay");
    });
  });

  describe("tracked projects", () => {
    it("shows each project's name, key and what the cache holds", async () => {
      mocked.syncOverview.mockResolvedValue(
        syncOverview({ projects: [projectCounts(), projectCounts({ id: "10002", key: "LUM", name: "Lumen iOS app", cached: 5, open: 2 })] }),
      );

      renderGroup();

      expect(await screen.findByText("Website relaunch")).toBeInTheDocument();
      expect(screen.getByText("NW")).toBeInTheDocument();
      expect(screen.getByText("33 issues · 21 open")).toBeInTheDocument();
      expect(screen.getByText("Lumen iOS app")).toBeInTheDocument();
      expect(screen.getByText("LUM")).toBeInTheDocument();
      expect(screen.getByText("5 issues · 2 open")).toBeInTheDocument();
    });

    it("uses singular for a single issue", async () => {
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts({ cached: 1, open: 1 })] }));

      renderGroup();

      expect(await screen.findByText("1 issue · 1 open")).toBeInTheDocument();
    });

    it("shows when a project last finished a full sync", async () => {
      const lastFullAt = "2026-03-04T09:30:00Z";
      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [projectCounts({ lastFullAt })] }));

      renderGroup();

      expect(await screen.findByText(new RegExp(`Synced at ${formatLocalTime(lastFullAt)}`))).toBeInTheDocument();
    });

    it("points to the Connection page when no project is tracked", async () => {
      renderGroup();

      expect(await screen.findByText("Track projects on the Connection page.")).toBeInTheDocument();
    });

    it("keeps the earlier rows and no empty-state text when a refetched overview lists no projects", async () => {
      mocked.syncOverview.mockResolvedValueOnce(syncOverview({ projects: [projectCounts({ name: "Northwind website" })] }));
      const client = createQueryClient();
      render(
        <QueryClientProvider client={client}>
          <CacheGroup />
        </QueryClientProvider>,
      );
      expect(await screen.findByText("Northwind website")).toBeInTheDocument();

      mocked.syncOverview.mockResolvedValue(syncOverview({ projects: [] }));
      await client.invalidateQueries({ queryKey: SYNC_KEY });

      await waitFor(() => expect(mocked.syncOverview).toHaveBeenCalledTimes(2));
      expect(screen.getByText("Northwind website")).toBeInTheDocument();
      expect(screen.getByText("NW")).toBeInTheDocument();
      expect(screen.queryByText("Track projects on the Connection page.")).not.toBeInTheDocument();
    });
  });
});
