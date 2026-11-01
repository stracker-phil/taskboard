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
import { chipDetail, chipText, SidebarFooter } from "./SidebarFooter";

const mocked = vi.mocked(api, true);

const NOW = new Date("2026-03-04T12:00:00Z");
const LAST_OK = "2026-03-04T11:58:00Z";
const LAST_FULL = "2026-03-04T09:30:00Z";
const RETRY_AT = "2026-03-04T12:05:00Z";

const progress = (patch: Partial<SyncProgress> = {}): SyncProgress => ({
  job: "delta",
  projectId: "10001",
  projectKey: "NW",
  pagesDone: 1,
  fetched: 2,
  approxTotal: null,
  ...patch,
});

async function renderFooter(state: SyncState) {
  mocked.syncOverview.mockResolvedValue(syncOverview({ state }));
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

const OFFLINE: SyncState = { state: "offline", since: LAST_OK, retryAt: RETRY_AT, lastOk: null };
const PAUSED: SyncState = { state: "paused", message: "Jira rejected the token." };
const FAILED: SyncState = { state: "error", kind: "internal", message: "Unexpected response.", since: LAST_OK, retryAt: RETRY_AT, lastOk: null };

describe("chipText", () => {
  it.each<[string, SyncState, SyncProgress[], string]>([
    ["an idle state never synced", { state: "idle", lastOk: null }, [], "Not synced yet"],
    ["a running full sync with progress", { state: "running", job: "full" }, [progress({ job: "full", fetched: 120 })], "Syncing…"],
    ["a running delta without progress", { state: "running", job: "delta" }, [], "Syncing…"],
    ["a running delta with one change", { state: "running", job: "delta" }, [progress({ fetched: 1 })], "Syncing… 1 change"],
    ["a running delta over two projects", { state: "running", job: "delta" }, [progress({ fetched: 2 }), progress({ projectId: "2", fetched: 1 })], "Syncing… 3 changes"],
    ["offline without a successful sync", OFFLINE, [], "Offline"],
    ["a pause", PAUSED, [], "Sync paused · token"],
    ["an error", FAILED, [], "Sync failed"],
  ])("words %s", (_name, state, prog, text) => {
    expect(chipText(state, prog, NOW)).toBe(text);
  });

  it("words the idle state relative to now", () => {
    expect(chipText({ state: "idle", lastOk: LAST_OK }, [], NOW)).toBe("Synced 2 min ago");
  });

  it("names the clock time of the last success when offline", () => {
    expect(chipText({ ...OFFLINE, lastOk: LAST_FULL }, [], NOW)).toBe(`Offline · synced ${clockTime(LAST_FULL)}`);
  });
});

describe("chipDetail", () => {
  it("names the reason and the next try when sync failed", () => {
    expect(chipDetail(FAILED)).toBe(`Unexpected response. Next try at ${clockTime(RETRY_AT)}; click to try now.`);
  });

  it("offers a retry when sync is paused", () => {
    expect(chipDetail(PAUSED)).toBe("Jira rejected the token. Click to try again.");
  });

  it("names since when Jira is unreachable and when the next try is when offline", () => {
    expect(chipDetail(OFFLINE)).toBe(
      `Jira can't be reached since ${clockTime(LAST_OK)}; the app works from the cache. Next try at ${clockTime(RETRY_AT)}; click to try now.`,
    );
  });

  it.each<[string, SyncState]>([
    ["idle", { state: "idle", lastOk: LAST_OK }],
    ["running", { state: "running", job: "delta" }],
  ])("offers to sync now when %s", (_name, state) => {
    expect(chipDetail(state)).toBe("Click to sync now (⌘R)");
  });
});

describe("SidebarFooter", () => {
  describe("the chip", () => {
    it.each<[string, SyncState, string, string]>([
      ["synced", { state: "idle", lastOk: new Date(Date.now() - 2 * 60_000).toISOString() }, "Synced 2 min ago", "Click to sync now (⌘R)"],
      ["never synced", { state: "idle", lastOk: null }, "Not synced yet", "Click to sync now (⌘R)"],
      ["syncing", { state: "running", job: "full" }, "Syncing…", "Click to sync now (⌘R)"],
      ["offline", { ...OFFLINE, lastOk: LAST_FULL }, `Offline · synced ${clockTime(LAST_FULL)}`, chipDetail(OFFLINE)],
      ["paused", PAUSED, "Sync paused · token", "Jira rejected the token. Click to try again."],
      ["failed", FAILED, "Sync failed", `Unexpected response. Next try at ${clockTime(RETRY_AT)}; click to try now.`],
    ])("shows the %s state with its data-state and tooltip", async (_name, state, text, tooltip) => {
      await renderFooter(state);

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
