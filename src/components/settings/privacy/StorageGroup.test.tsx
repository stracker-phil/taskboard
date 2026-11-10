import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore, useStore } from "../../../state/store";
import { syncOverview } from "../../../test/jira";
import { defaultSync, settings } from "../../../test/settings";
import { StorageGroup } from "./StorageGroup";

const mocked = vi.mocked(api, true);
const toasts = () => useStore.getState().toasts.map((t) => t.message);
const IN_MEMORY = settings({ sync: { ...defaultSync(), memoryOnly: true } });

function renderGroup() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StorageGroup />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.syncOverview.mockResolvedValue(syncOverview({ cacheBytes: 12_500_000 }));
  mocked.getSettings.mockResolvedValue(settings());
});

describe("StorageGroup", () => {
  it("puts the memory-only switch first", async () => {
    renderGroup();

    const switches = await screen.findAllByRole("switch");

    expect(switches[0]).toHaveAccessibleName("Keep Jira data in memory only");
  });

  describe("data folder", () => {
    it("opens the data folder through the app:data command", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("button", { name: "Show Data Folder" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:data");
    });
  });

  describe("copy on disk", () => {
    it("says how big the copy of the issues is and that it can be reset", async () => {
      renderGroup();

      expect(await screen.findByText("Copy of your Jira issues")).toBeInTheDocument();
      expect(screen.getByText("12.5 MB on this Mac. Reset it when issues look out of date.")).toBeInTheDocument();
    });

    it("leaves the size out until the overview has loaded", async () => {
      mocked.syncOverview.mockReturnValue(new Promise(() => {}));

      renderGroup();

      expect(await screen.findByText("Reset it when issues look out of date.")).toBeInTheDocument();
    });

    it("asks before resetting and says what stays", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("button", { name: "Reset Cache" }));

      const dialog = screen.getByRole("alertdialog", { name: "Reset the cache?" });
      expect(dialog).toHaveTextContent("Spaces, filter lists, views, bookmarks and what you've read stay");
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("closes without resetting when canceled", async () => {
      renderGroup();
      await userEvent.click(await screen.findByRole("button", { name: "Reset Cache" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("resets the cache when confirmed", async () => {
      renderGroup();
      await userEvent.click(await screen.findByRole("button", { name: "Reset Cache" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Reset Cache" }));

      expect(mocked.cacheReset).toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      await waitFor(() => expect(toasts()).toContain("The cache is empty; the next sync fetches your issues again."));
    });

    it("shows an error toast when the reset fails", async () => {
      mocked.cacheReset.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderGroup();
      await userEvent.click(await screen.findByRole("button", { name: "Reset Cache" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Reset Cache" }));

      await waitFor(() => expect(toasts()).toContain("Could not reset the cache: Disk full"));
    });
  });

  describe("data held in memory", () => {
    beforeEach(() => {
      mocked.getSettings.mockResolvedValue(IN_MEMORY);
    });

    it("says how much memory the Jira data takes while Taskboard runs", async () => {
      renderGroup();

      expect(await screen.findByText("Jira data in memory")).toBeInTheDocument();
      expect(screen.getByText("12.5 MB while Taskboard runs.")).toBeInTheDocument();
      expect(screen.queryByText("Copy of your Jira issues")).not.toBeInTheDocument();
    });

    it("offers Refetch from Jira instead of Reset Cache", async () => {
      renderGroup();

      expect(await screen.findByRole("button", { name: "Refetch from Jira" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Reset Cache" })).not.toBeInTheDocument();
    });

    it("asks before refetching and says what stays", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("button", { name: "Refetch from Jira" }));

      const dialog = screen.getByRole("alertdialog", { name: "Refetch from Jira?" });
      expect(dialog).toHaveTextContent("Spaces, filter lists, views, bookmarks and what you've read stay");
      expect(within(dialog).getByRole("button", { name: "Refetch" })).toBeInTheDocument();
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("drops the data when Refetch is confirmed", async () => {
      renderGroup();
      await userEvent.click(await screen.findByRole("button", { name: "Refetch from Jira" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Refetch" }));

      expect(mocked.cacheReset).toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      await waitFor(() => expect(toasts()).toContain("Taskboard fetches your issues again."));
    });

    it("shows an error toast when the refetch fails", async () => {
      mocked.cacheReset.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderGroup();
      await userEvent.click(await screen.findByRole("button", { name: "Refetch from Jira" }));

      await userEvent.click(within(screen.getByRole("alertdialog")).getByRole("button", { name: "Refetch" }));

      await waitFor(() => expect(toasts()).toContain("Could not refetch from Jira: Disk full"));
    });

    it("shows neither keep-history nor clear-on-quit", async () => {
      renderGroup();

      await screen.findByRole("button", { name: "Refetch from Jira" });

      expect(screen.queryByRole("group", { name: "Keep history you've read" })).not.toBeInTheDocument();
      expect(screen.queryByRole("switch", { name: "Clear the cache when Taskboard quits" })).not.toBeInTheDocument();
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
      mocked.getSettings.mockResolvedValue(IN_MEMORY);

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

      expect(await screen.findByRole("switch", { name: "Clear the cache when Taskboard quits" })).toBeInTheDocument();
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
});
