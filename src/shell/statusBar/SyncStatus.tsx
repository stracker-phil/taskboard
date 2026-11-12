import { RotateCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useLocalDays } from "../../queries/localDays";
import { useSyncOverview, useSyncProgress } from "../../queries/sync";
import { runBusCommand } from "../../state/requests";
import { syncDetail, syncText } from "./syncWords";

/** Re-renders every 30 s so "2 min ago" keeps counting. */
function useTicker(ms = 30_000): Date {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), ms);
    return () => clearInterval(timer);
  }, [ms]);
  return now;
}

/** The sync state at the status bar's right end: a dot, the state in words and a refresh icon; a click syncs now
 *  or retries. */
export function SyncStatus() {
  const { data: overview } = useSyncOverview();
  const progress = useSyncProgress();
  const now = useTicker();
  const days = useLocalDays();
  const state = overview?.state;
  return (
    <button
      type="button"
      className="status-sync"
      data-state={state?.state}
      title={state ? syncDetail(state, overview.lastFullAt) : undefined}
      onClick={() => runBusCommand("sync:refresh", "Could not sync")}
    >
      <span className="status-sync__dot" data-state={state?.state} aria-hidden="true" />
      <span className="status-sync__text">{state ? syncText(state, Object.values(progress), now, days) : ""}</span>
      <RotateCw size={13} strokeWidth={2} aria-hidden="true" className={state?.state === "running" ? "status-bar__spin" : undefined} />
    </button>
  );
}
