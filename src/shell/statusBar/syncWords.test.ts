import { describe, expect, it } from "vitest";
import type { SyncProgress, SyncState } from "../../api";
import { clockTime, fullDateTime } from "../../lib/relativeTime";
import { BERLIN_DAYS } from "../../test/days";
import { syncDetail, syncText } from "./syncWords";

const NOW = new Date("2026-11-12T11:00:00Z");
const LAST_OK = "2026-11-12T10:58:00Z";
const SINCE = "2026-11-12T10:40:00Z";
const LAST_FULL = "2026-11-12T08:30:00Z";
const RETRY_AT = "2026-11-12T11:05:00Z";

const progress = (patch: Partial<SyncProgress> = {}): SyncProgress => ({
  job: "delta",
  projectId: "10001",
  projectKey: "NW",
  pagesDone: 1,
  fetched: 2,
  approxTotal: null,
  ...patch,
});

const OFFLINE: SyncState = { state: "offline", since: SINCE, retryAt: RETRY_AT, lastOk: null };
const PAUSED: SyncState = { state: "paused", message: "Jira rejected the token." };
const FAILED: SyncState = { state: "error", kind: "internal", message: "Unexpected response.", since: SINCE, retryAt: RETRY_AT, lastOk: null };

describe("syncText", () => {
  it.each<[string, SyncState, SyncProgress[], string]>([
    ["an idle state never synced", { state: "idle", lastOk: null }, [], "Not synced yet"],
    ["an idle state synced two minutes ago", { state: "idle", lastOk: LAST_OK }, [], "Synced 2 min ago"],
    ["a running full sync with progress", { state: "running", job: "full" }, [progress({ job: "full", fetched: 120 })], "Syncing…"],
    ["a running delta without progress", { state: "running", job: "delta" }, [], "Syncing…"],
    ["a running delta with one change", { state: "running", job: "delta" }, [progress({ fetched: 1 })], "Syncing… 1 change"],
    ["a running delta over two projects", { state: "running", job: "delta" }, [progress({ fetched: 2 }), progress({ projectId: "2", fetched: 1 })], "Syncing… 3 changes"],
    ["offline without a successful sync", OFFLINE, [], "Offline"],
    ["offline after a successful sync", { ...OFFLINE, lastOk: LAST_OK }, [], "Offline · synced 2 min ago"],
    ["a pause", PAUSED, [], "Sync paused · token"],
    ["an error", FAILED, [], "Sync failed"],
  ])("words %s", (_name, state, prog, text) => {
    expect(syncText(state, prog, NOW, BERLIN_DAYS)).toBe(text);
  });

  it("words an older success of today with its clock time in the owner's zone", () => {
    const clock = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit", timeZone: "Europe/Berlin" }).format(new Date(LAST_FULL));

    expect(syncText({ state: "idle", lastOk: LAST_FULL }, [], NOW, BERLIN_DAYS)).toBe(`Synced at ${clock}`);
  });

  it("words a success from the day before as yesterday", () => {
    expect(syncText({ state: "idle", lastOk: "2026-11-11T09:00:00Z" }, [], NOW, BERLIN_DAYS)).toMatch(/^Synced yesterday at /);
  });
});

describe("syncDetail", () => {
  it("names the reason and the next try when sync failed, and offers to try now", () => {
    expect(syncDetail(FAILED, null)).toBe(`Unexpected response. Next try at ${clockTime(RETRY_AT)}.\nClick to try now (⌘R)`);
  });

  it("gives the reason when sync is paused, and offers to try now", () => {
    expect(syncDetail(PAUSED, null)).toBe("Jira rejected the token.\nClick to try now (⌘R)");
  });

  it("names since when Jira is unreachable and when the next try is, when offline", () => {
    expect(syncDetail(OFFLINE, null)).toBe(
      `Jira can't be reached since ${clockTime(SINCE)}; the app works from the cache. Next try at ${clockTime(RETRY_AT)}.\nClick to try now (⌘R)`,
    );
  });

  it("lists the last sync and the last full sync in full after the reason", () => {
    const detail = syncDetail({ ...FAILED, lastOk: LAST_OK }, LAST_FULL);

    expect(detail.split("\n")).toEqual([
      `Unexpected response. Next try at ${clockTime(RETRY_AT)}.`,
      `Last sync: ${fullDateTime(LAST_OK)}`,
      `Last full sync: ${fullDateTime(LAST_FULL)}`,
      "Click to try now (⌘R)",
    ]);
  });

  it("lists the last sync of an idle state and offers to sync now", () => {
    expect(syncDetail({ state: "idle", lastOk: LAST_OK }, null)).toBe(`Last sync: ${fullDateTime(LAST_OK)}\nClick to sync now (⌘R)`);
  });

  it("offers only to sync now when nothing ever synced", () => {
    expect(syncDetail({ state: "idle", lastOk: null }, null)).toBe("Click to sync now (⌘R)");
  });

  it("shows the last full sync but no last sync while running", () => {
    expect(syncDetail({ state: "running", job: "delta" }, LAST_FULL)).toBe(`Last full sync: ${fullDateTime(LAST_FULL)}\nClick to sync now (⌘R)`);
  });
});
