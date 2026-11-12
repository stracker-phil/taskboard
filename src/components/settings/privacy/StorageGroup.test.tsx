import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore, useStore } from "../../../state/store";
import { syncOverview } from "../../../test/jira";
import { StorageGroup } from "./StorageGroup";

const mocked = vi.mocked(api, true);
const toasts = () => useStore.getState().toasts.map((t) => t.message);

function renderGroup() {
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StorageGroup />
    </QueryClientProvider>,
  );
}

async function openRefetchSheet() {
  await userEvent.click(await screen.findByRole("button", { name: "Refetch from Jira" }));
  return screen.getByRole("alertdialog", { name: "Refetch from Jira?" });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  mocked.syncOverview.mockResolvedValue(syncOverview({ cacheBytes: 12_500_000 }));
});

describe("StorageGroup", () => {
  it("says that nothing from Jira is written to disk", async () => {
    renderGroup();

    expect(await screen.findByText("Nothing from Jira is written to disk; it is gone when Taskboard quits.")).toBeInTheDocument();
  });

  it("offers no storage switches or disk cache controls", async () => {
    renderGroup();
    await screen.findByRole("button", { name: "Refetch from Jira" });

    expect(screen.queryByRole("switch")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Reset Cache" })).not.toBeInTheDocument();
    expect(screen.queryByRole("group", { name: "Keep history you've read" })).not.toBeInTheDocument();
  });

  describe("data folder", () => {
    it("opens the data folder through the app:data command", async () => {
      renderGroup();

      await userEvent.click(await screen.findByRole("button", { name: "Show Data Folder" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:data");
    });
  });

  describe("data held in memory", () => {
    it("says how much memory the Jira data takes while Taskboard runs", async () => {
      renderGroup();

      expect(await screen.findByText("Jira data in memory")).toBeInTheDocument();
      expect(await screen.findByText("12.5 MB while Taskboard runs.")).toBeInTheDocument();
    });

    it("leaves the size out until the overview has loaded", async () => {
      mocked.syncOverview.mockReturnValue(new Promise(() => {}));

      renderGroup();

      expect(await screen.findByText("Jira data in memory")).toBeInTheDocument();
      expect(screen.queryByText(/while Taskboard runs/)).not.toBeInTheDocument();
    });

    it("asks before refetching and says what stays", async () => {
      renderGroup();

      const dialog = await openRefetchSheet();

      expect(dialog).toHaveTextContent("Spaces, filter lists, views, bookmarks and what you've read stay");
      expect(within(dialog).getByRole("button", { name: "Refetch" })).toBeInTheDocument();
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("closes without refetching when canceled", async () => {
      renderGroup();
      const dialog = await openRefetchSheet();

      await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));

      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      expect(mocked.cacheReset).not.toHaveBeenCalled();
    });

    it("drops the data and tells that the issues are fetched again when Refetch is confirmed", async () => {
      renderGroup();
      const dialog = await openRefetchSheet();

      await userEvent.click(within(dialog).getByRole("button", { name: "Refetch" }));

      expect(mocked.cacheReset).toHaveBeenCalled();
      expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
      await waitFor(() => expect(toasts()).toContain("Taskboard fetches your issues again."));
    });

    it("shows an error toast and no success toast when the refetch fails", async () => {
      mocked.cacheReset.mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
      renderGroup();
      const dialog = await openRefetchSheet();

      await userEvent.click(within(dialog).getByRole("button", { name: "Refetch" }));

      await waitFor(() => expect(toasts()).toContain("Could not refetch from Jira: Disk full"));
      expect(toasts()).not.toContain("Taskboard fetches your issues again.");
    });
  });
});
