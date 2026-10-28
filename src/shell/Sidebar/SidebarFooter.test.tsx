import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type SyncProgress, type SyncState } from "../../api";
import { clockTime } from "../../lib/relativeTime";
import { createQueryClient } from "../../queries/queryClient";
import { resetStore } from "../../state/store";
import { emitEvent } from "../../test/apiMock";
import { syncOverview } from "../../test/jira";
import { chipText, SidebarFooter } from "./SidebarFooter";

const mocked = vi.mocked(api, true);

const NOW = new Date("2026-03-04T12:00:00Z");
const LAST_OK = "2026-03-04T11:58:00Z";
const LAST_FULL = "2026-03-04T09:30:00Z";

const progress = (patch: Partial<SyncProgress> = {}): SyncProgress => ({
  job: "delta",
  projectId: "10001",
  projectKey: "NW",
  pagesDone: 1,
  fetched: 2,
  approxTotal: null,
  ...patch,
});

async function renderFooter(state: SyncState, lastFullAt: string | null = null) {
  mocked.syncOverview.mockResolvedValue(syncOverview({ state, lastFullAt }));
  render(
    <QueryClientProvider client={createQueryClient()}>
      <SidebarFooter />
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

describe("chipText", () => {
  it.each<[string, SyncState, string | null, SyncProgress[], string]>([
    ["an idle state never synced", { state: "idle", lastOk: null }, null, [], "Not synced yet"],
    ["a running full sync with progress", { state: "running", job: "full" }, null, [progress({ job: "full", fetched: 120 })], "Syncing…"],
    ["a running delta without progress", { state: "running", job: "delta" }, null, [], "Syncing…"],
    ["a running delta with one change", { state: "running", job: "delta" }, null, [progress({ fetched: 1 })], "Syncing… 1 change"],
    ["a running delta over two projects", { state: "running", job: "delta" }, null, [progress({ fetched: 2 }), progress({ projectId: "2", fetched: 1 })], "Syncing… 3 changes"],
    ["offline without a full sync", { state: "offline", since: LAST_FULL }, null, [], "Offline"],
    ["a pause", { state: "paused", message: "Jira rejected the token." }, null, [], "Sync paused · token"],
    ["an error", { state: "error", kind: "internal", message: "Boom." }, null, [], "Sync failed"],
  ])("words %s", (_name, state, lastOk, prog, text) => {
    expect(chipText(state, lastOk, prog, NOW)).toBe(text);
  });

  it("words the idle state relative to now", () => {
    expect(chipText({ state: "idle", lastOk: LAST_OK }, null, [], NOW)).toBe("Synced 2 min ago");
  });

  it("names the clock time of the last full sync when offline", () => {
    expect(chipText({ state: "offline", since: LAST_OK }, LAST_FULL, [], NOW)).toBe(`Offline · cache from ${clockTime(LAST_FULL)}`);
  });
});

describe("SidebarFooter", () => {
  describe("the chip", () => {
    it.each<[string, SyncState, string | null, string, string]>([
      ["synced", { state: "idle", lastOk: new Date(Date.now() - 2 * 60_000).toISOString() }, null, "Synced 2 min ago", "Click to sync now (⌘R)"],
      ["never synced", { state: "idle", lastOk: null }, null, "Not synced yet", "Click to sync now (⌘R)"],
      ["syncing", { state: "running", job: "full" }, null, "Syncing…", "Click to sync now (⌘R)"],
      [
        "offline",
        { state: "offline", since: LAST_OK },
        LAST_FULL,
        `Offline · cache from ${clockTime(LAST_FULL)}`,
        "Jira can't be reached; the app works from the cache and retries every minute. Click to try now.",
      ],
      ["paused", { state: "paused", message: "Jira rejected the token." }, null, "Sync paused · token", "Jira rejected the token. Click to try again."],
      ["failed", { state: "error", kind: "internal", message: "Unexpected response." }, null, "Sync failed", "Unexpected response. Click to try again."],
    ])("shows the %s state with its data-state and tooltip", async (_name, state, lastFullAt, text, tooltip) => {
      await renderFooter(state, lastFullAt);

      const button = await chip(text);

      expect(button).toHaveAttribute("data-state", state.state);
      expect(button).toHaveAttribute("title", tooltip);
    });

    it("runs the sync:refresh command when clicked", async () => {
      await renderFooter({ state: "idle", lastOk: null });

      await userEvent.click(await chip("Not synced yet"));

      expect(mocked.runCommand).toHaveBeenCalledWith("sync:refresh");
    });

    it("keeps counting up every 30 seconds", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true, now: NOW });
      await renderFooter({ state: "idle", lastOk: LAST_OK });
      expect(await chip("Synced 2 min ago")).toBeInTheDocument();

      await act(() => vi.advanceTimersByTimeAsync(60_000));

      expect(await chip("Synced 3 min ago")).toBeInTheDocument();
    });
  });

  describe("during a delta sync", () => {
    it("counts the fetched changes from sync_progress events", async () => {
      await renderFooter({ state: "running", job: "delta" });
      await chip("Syncing…");

      act(() => {
        emitEvent(api, "sync_progress", progress({ fetched: 2 }));
        emitEvent(api, "sync_progress", progress({ projectId: "10002", projectKey: "LUM", fetched: 1 }));
      });

      expect(await chip("Syncing… 3 changes")).toBeInTheDocument();
    });

    it("shows no count for a full sync", async () => {
      await renderFooter({ state: "running", job: "full" });
      await chip("Syncing…");

      act(() => emitEvent(api, "sync_progress", progress({ job: "full", fetched: 120 })));

      expect(await chip("Syncing…")).toBeInTheDocument();
      expect(screen.queryByText(/change/)).not.toBeInTheDocument();
    });
  });

  describe("the Settings button", () => {
    it("runs the app:settings command", async () => {
      await renderFooter({ state: "idle", lastOk: null });

      await userEvent.click(screen.getByRole("button", { name: "Settings" }));

      expect(mocked.runCommand).toHaveBeenCalledWith("app:settings");
    });
  });
});
