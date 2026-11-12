import { describe, expect, it } from "vitest";
import type { SyncState } from "../api";
import { formatLocalTime } from "../lib/time";
import { BERLIN_DAYS } from "../test/days";
import { describeSyncState } from "./sync";

describe("describeSyncState", () => {
  it.each<[string, SyncState, string]>([
    ["an idle state that never synced", { state: "idle", lastOk: null }, "Not synced yet"],
    ["a running sync", { state: "running", job: "full" }, "Syncing…"],
    [
      "an error",
      {
        state: "error",
        kind: "auth",
        message: "Jira rejected the token.",
        since: "2026-10-28T10:00:00.000Z",
        retryAt: "2026-10-28T10:05:00.000Z",
        lastOk: null,
      },
      "Jira rejected the token.",
    ],
    ["a paused sync", { state: "paused", message: "Reconnect Jira to continue." }, "Reconnect Jira to continue."],
  ])("describes %s", (_name, state, expected) => {
    expect(describeSyncState(state, BERLIN_DAYS)).toBe(expected);
  });

  it("names the time of the last successful sync when idle", () => {
    const lastOk = "2026-11-12T08:30:00Z";

    expect(describeSyncState({ state: "idle", lastOk }, BERLIN_DAYS)).toBe(`Synced at ${formatLocalTime(lastOk, BERLIN_DAYS)}`);
  });

  it("names the date too when the last successful sync was not today", () => {
    const lastOk = "2026-11-10T08:30:00Z";

    expect(describeSyncState({ state: "idle", lastOk }, BERLIN_DAYS)).toContain("2026");
  });

  it("names the time the connection was lost when offline", () => {
    const since = "2026-11-12T08:30:00Z";

    expect(describeSyncState({ state: "offline", since, retryAt: "2026-11-12T08:35:00Z", lastOk: null }, BERLIN_DAYS)).toBe(
      `Offline since ${formatLocalTime(since, BERLIN_DAYS)}`,
    );
  });
});
