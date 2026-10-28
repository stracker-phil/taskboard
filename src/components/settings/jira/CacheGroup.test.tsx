import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../../test/apiMock")).createApiMock() }));

import { api } from "../../../api";
import { formatLocalTime } from "../../../lib/time";
import { createQueryClient } from "../../../queries/queryClient";
import { resetStore } from "../../../state/store";
import { projectCounts, syncOverview } from "../../../test/jira";
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
  });
});
