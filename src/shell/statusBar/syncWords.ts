import type { LocalDays, SyncProgress, SyncState } from "../../api";
import { clockTime, fullDateTime, shortWhen } from "../../lib/relativeTime";

/** The status bar's line for each sync state. */
export function syncText(state: SyncState, progress: SyncProgress[], now: Date, days: LocalDays | undefined): string {
  switch (state.state) {
    case "idle":
      return state.lastOk ? `Synced ${shortWhen(state.lastOk, now, days)}` : "Not synced yet";
    case "running": {
      const changes = state.job === "delta" ? progress.reduce((sum, p) => sum + p.fetched, 0) : 0;
      return changes > 0 ? `Syncing… ${changes} ${changes === 1 ? "change" : "changes"}` : "Syncing…";
    }
    case "offline":
      return state.lastOk ? `Offline · synced ${shortWhen(state.lastOk, now, days)}` : "Offline";
    case "paused":
      return "Sync paused · token";
    case "error":
      return "Sync failed";
  }
}

/** The tooltip: why it failed and when it tries again, the last syncs to the second, and what a click does. */
export function syncDetail(state: SyncState, lastFullAt: string | null): string {
  const lines: string[] = [];
  switch (state.state) {
    case "error":
      lines.push(`${state.message} Next try at ${clockTime(state.retryAt)}.`);
      break;
    case "paused":
      lines.push(state.message);
      break;
    case "offline":
      lines.push(`Jira can't be reached since ${clockTime(state.since)}; the app works from the cache. Next try at ${clockTime(state.retryAt)}.`);
      break;
  }
  const lastOk = state.state === "idle" || state.state === "error" || state.state === "offline" ? state.lastOk : null;
  if (lastOk) lines.push(`Last sync: ${fullDateTime(lastOk)}`);
  if (lastFullAt) lines.push(`Last full sync: ${fullDateTime(lastFullAt)}`);
  lines.push(state.state === "idle" || state.state === "running" ? "Click to sync now (⌘R)" : "Click to try now (⌘R)");
  return lines.join("\n");
}
