import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, type SyncProgress, type SyncState } from "../api";
import { formatLocalTime } from "../lib/time";

export const SYNC_KEY = ["sync"] as const;

/** The sync state and what the cache holds per tracked project; `sync_state` and `issues_changed` refresh it. */
export const useSyncOverview = () => useQuery({ queryKey: SYNC_KEY, queryFn: api.syncOverview });

/** The latest `sync_progress` per project id of the running full sync; empty once the sync stops. */
export function useSyncProgress(): Record<string, SyncProgress> {
  const [progress, setProgress] = useState<Record<string, SyncProgress>>({});
  useEffect(() => {
    const unlisten = [
      api.listen("sync_progress", (p) => setProgress((prev) => ({ ...prev, [p.projectId]: p }))),
      api.listen("sync_state", (s) => {
        if (s.state !== "running") setProgress({});
      }),
    ];
    return () => unlisten.forEach((p) => void p.then((fn) => fn()).catch(() => {}));
  }, []);
  return progress;
}

/** One line for the sync state: "Synced at …", "Syncing…", "Offline since …" or Rust's message. */
export function describeSyncState(state: SyncState): string {
  switch (state.state) {
    case "idle":
      return state.lastOk ? `Synced at ${formatLocalTime(state.lastOk)}` : "Not synced yet";
    case "running":
      return "Syncing…";
    case "offline":
      return `Offline since ${formatLocalTime(state.since)}`;
    case "error":
    case "paused":
      return state.message;
  }
}
