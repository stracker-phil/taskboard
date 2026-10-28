import { describe, expect, it } from "vitest";
import type { SyncState } from "../api";
import { formatLocalTime } from "../lib/time";
import { describeSyncState } from "./sync";

describe("describeSyncState", () => {
  it.each<[string, SyncState, string]>([
    ["an idle state that never synced", { state: "idle", lastOk: null }, "Not synced yet"],
    ["a running sync", { state: "running", job: "full" }, "Syncing…"],
    ["an error", { state: "error", kind: "auth", message: "Jira rejected the token." }, "Jira rejected the token."],
    ["a paused sync", { state: "paused", message: "Reconnect Jira to continue." }, "Reconnect Jira to continue."],
  ])("describes %s", (_name, state, expected) => {
    expect(describeSyncState(state)).toBe(expected);
  });

  it("names the time of the last successful sync when idle", () => {
    const lastOk = "2026-03-04T09:30:00Z";

    expect(describeSyncState({ state: "idle", lastOk })).toBe(`Synced at ${formatLocalTime(lastOk)}`);
  });

  it("names the time the connection was lost when offline", () => {
    const since = "2026-03-04T09:30:00Z";

    expect(describeSyncState({ state: "offline", since })).toBe(`Offline since ${formatLocalTime(since)}`);
  });
});
