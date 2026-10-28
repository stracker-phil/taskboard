import { RotateCw, Settings } from "lucide-react";
import { api } from "../../api";
import { describeSyncState, useSyncOverview } from "../../queries/sync";
import { attempt, runBusCommand } from "../../state/requests";

/** The sync chip (its state in words; a click syncs now) and the Settings button. */
export function SidebarFooter() {
  const { data: overview } = useSyncOverview();
  const state = overview?.state;
  return (
    <div className="sb-footer">
      <button type="button" className="sb-sync" title="Sync Now" onClick={() => void attempt(api.syncRun(), "Could not sync")}>
        <span className="sb-sync__dot" data-state={state?.state} aria-hidden="true" />
        <span className="sb-sync__text">{state ? describeSyncState(state) : ""}</span>
        <RotateCw size={14} strokeWidth={2} aria-hidden="true" />
      </button>
      <button type="button" className="sb-settings" aria-label="Settings" title="Settings (⌘,)" onClick={() => runBusCommand("app:settings", "Could not open Settings")}>
        <Settings size={17} strokeWidth={1.8} aria-hidden="true" />
      </button>
    </div>
  );
}
