import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type SyncOverview, type SyncProgress, type SyncState } from "../../api";
import { fullDateTime } from "../../lib/relativeTime";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { emitEvent } from "../../test/apiMock";
import { projectCounts, syncOverview } from "../../test/jira";
import { defaultSync, settings } from "../../test/settings";
import { StatusBar } from "./StatusBar";

const mocked = vi.mocked(api, true);

const NOW = new Date(2026, 2, 4, 12, 0);
const LAST_OK = new Date(2026, 2, 4, 11, 58).toISOString();
const SINCE = new Date(2026, 2, 4, 11, 40).toISOString();
const LAST_FULL = new Date(2026, 2, 4, 9, 30).toISOString();
const RETRY_AT = new Date(2026, 2, 4, 12, 5).toISOString();

const OFFLINE: SyncState = { state: "offline", since: SINCE, retryAt: RETRY_AT, lastOk: null };
const PAUSED: SyncState = { state: "paused", message: "Jira rejected the token." };
const FAILED: SyncState = { state: "error", kind: "internal", message: "Unexpected response.", since: SINCE, retryAt: RETRY_AT, lastOk: null };

const progress = (patch: Partial<SyncProgress> = {}): SyncProgress => ({
  job: "delta",
  projectId: "10001",
  projectKey: "NW",
  pagesDone: 1,
  fetched: 2,
  approxTotal: null,
  ...patch,
});

async function renderBar(overview: Partial<SyncOverview> = {}, memoryOnly = false) {
  mocked.syncOverview.mockResolvedValue(syncOverview(overview));
  mocked.getSettings.mockResolvedValue(settings({ sync: { ...defaultSync(), memoryOnly } }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <StatusBar />
    </QueryClientProvider>,
  );
  await waitFor(() => expect(mocked.listen).toHaveBeenCalledWith("sync_progress", expect.any(Function)));
}

const chip = (name: string | RegExp) => screen.findByRole("button", { name });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("StatusBar", () => {
  describe("the sync state", () => {
    it.each<[string, SyncState, string]>([
      ["synced", { state: "idle", lastOk: LAST_OK }, "Synced 2 min ago"],
      ["never synced", { state: "idle", lastOk: null }, "Not synced yet"],
      ["syncing", { state: "running", job: "full" }, "Syncing…"],
      ["offline", { ...OFFLINE, lastOk: LAST_OK }, "Offline · synced 2 min ago"],
      ["paused", PAUSED, "Sync paused · token"],
      ["failed", FAILED, "Sync failed"],
    ])("shows the %s state with its data-state", async (_name, state, text) => {
      vi.useFakeTimers({ shouldAdvanceTime: true, now: NOW });
      await renderBar({ state });

      expect(await chip(text)).toHaveAttribute("data-state", state.state);
    });

    it.each<[string, SyncState, string, string]>([
      ["idle", { state: "idle", lastOk: null }, "Not synced yet", "Click to sync now (⌘R)"],
      ["running", { state: "running", job: "full" }, "Syncing…", "Click to sync now (⌘R)"],
      ["paused", PAUSED, "Sync paused · token", "Jira rejected the token.\nClick to try now (⌘R)"],
    ])("explains the %s state in the tooltip", async (_name, state, text, tooltip) => {
      await renderBar({ state });

      expect(await chip(text)).toHaveAttribute("title", tooltip);
    });

    it("adds the last sync and the last full sync to the tooltip in full", async () => {
      await renderBar({ state: { state: "idle", lastOk: LAST_OK }, lastFullAt: LAST_FULL });

      const button = await screen.findByRole("button", { name: /^Synced/ });

      expect(button).toHaveAttribute("title", `Last sync: ${fullDateTime(LAST_OK)}\nLast full sync: ${fullDateTime(LAST_FULL)}\nClick to sync now (⌘R)`);
    });

    it("runs the sync:refresh command when clicked", async () => {
      await renderBar();

      await userEvent.click(await chip("Not synced yet"));

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
    });

    it("keeps counting up every 30 seconds", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true, now: NOW });
      await renderBar({ state: { state: "idle", lastOk: LAST_OK } });
      expect(await chip("Synced 2 min ago")).toBeInTheDocument();

      await act(() => vi.advanceTimersByTimeAsync(60_000));

      expect(await chip("Synced 3 min ago")).toBeInTheDocument();
    });

    describe("during a delta sync", () => {
      it("counts the fetched changes from sync_progress events", async () => {
        await renderBar({ state: { state: "running", job: "delta" } });
        await chip("Syncing…");

        act(() => {
          emitEvent(api, "sync_progress", progress({ fetched: 2 }));
          emitEvent(api, "sync_progress", progress({ projectId: "10002", projectKey: "LUM", fetched: 1 }));
        });

        expect(await chip("Syncing… 3 changes")).toBeInTheDocument();
      });

      it("shows no count for a full sync", async () => {
        await renderBar({ state: { state: "running", job: "full" } });
        await chip("Syncing…");

        act(() => emitEvent(api, "sync_progress", progress({ job: "full", fetched: 120 })));

        expect(await chip("Syncing…")).toBeInTheDocument();
        expect(screen.queryByText(/change/)).not.toBeInTheDocument();
      });
    });
  });

  describe("the memory chip", () => {
    it("names the cached issues and the size of the cache while the data lives in memory only", async () => {
      await renderBar({ projects: [projectCounts({ cached: 33 }), projectCounts({ id: "10002", key: "LUM", cached: 9 })], cacheBytes: 1_540_000 }, true);

      const icon = await screen.findByRole("img", { name: "Jira data in memory" });

      expect(icon).toHaveAttribute("title", expect.stringContaining("42 issues · 1.5 MB"));
    });

    it("says issue in the singular for a single cached issue", async () => {
      await renderBar({ projects: [projectCounts({ cached: 1 })], cacheBytes: 0 }, true);

      expect(await screen.findByRole("img", { name: "Jira data in memory" })).toHaveAttribute("title", expect.stringContaining("1 issue · 0.0 MB"));
    });

    it("is absent while the data is stored on disk", async () => {
      await renderBar({ projects: [projectCounts()] }, false);
      await chip("Not synced yet");

      expect(screen.queryByRole("img", { name: "Jira data in memory" })).not.toBeInTheDocument();
    });
  });
});
