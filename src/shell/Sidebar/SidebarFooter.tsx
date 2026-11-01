import { RotateCw, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import type { SyncProgress, SyncState } from "../../api";
import { clockTime, timeAgo } from "../../lib/relativeTime";
import { useSyncOverview, useSyncProgress } from "../../queries/sync";
import { runBusCommand } from "../../state/requests";

/** Re-renders every 30 s so "2 min ago" keeps counting. */
function useTicker(ms = 30_000): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), ms);
    return () => clearInterval(timer);
  }, [ms]);
  return now;
}

/** The chip's line for each state (artboard 8, panel 5). */
export function chipText(state: SyncState, progress: SyncProgress[], now: Date): string {
  switch (state.state) {
    case "idle":
      return state.lastOk ? `Synced ${timeAgo(state.lastOk, now)}` : "Not synced yet";
    case "running": {
      const changes = state.job === "delta" ? progress.reduce((sum, p) => sum + p.fetched, 0) : 0;
      return changes > 0 ? `Syncing… ${changes} ${changes === 1 ? "change" : "changes"}` : "Syncing…";
    }
    case "offline":
      return state.lastOk ? `Offline · synced ${clockTime(state.lastOk)}` : "Offline";
    case "paused":
      return "Sync paused · token";
    case "error":
      return "Sync failed";
  }
}

/** The detail behind the chip, as its tooltip: why it failed and when it tries again. */
export function chipDetail(state: SyncState): string {
  switch (state.state) {
    case "error":
      return `${state.message} Next try at ${clockTime(state.retryAt)}; click to try now.`;
    case "paused":
      return `${state.message} Click to try again.`;
    case "offline":
      return `Jira can't be reached since ${clockTime(state.since)}; the app works from the cache. Next try at ${clockTime(state.retryAt)}; click to try now.`;
    default:
      return "Click to sync now (⌘R)";
  }
}

/** The sync chip (its state; a click syncs now or retries) and the Settings button. */
export function SidebarFooter() {
  const { data: overview } = useSyncOverview();
  const progress = useSyncProgress();
  const now = useTicker();
  const state = overview?.state;
  return (
    <div className="sb-footer">
      <button
        type="button"
        className="sb-sync"
        data-state={state?.state}
        title={state ? chipDetail(state) : undefined}
        onClick={() => runBusCommand("sync:refresh", "Could not sync")}
      >
        <span className="sb-sync__dot" data-state={state?.state} aria-hidden="true" />
        <span className="sb-sync__text">{state ? chipText(state, Object.values(progress), now) : ""}</span>
        <RotateCw size={14} strokeWidth={2} aria-hidden="true" className={state?.state === "running" ? "sb-sync__spin" : undefined} />
      </button>
      <button type="button" className="sb-settings" aria-label="Settings" title="Settings (⌘,)" onClick={() => runBusCommand("app:settings", "Could not open Settings")}>
        <Settings size={17} strokeWidth={1.8} aria-hidden="true" />
      </button>
    </div>
  );
}
