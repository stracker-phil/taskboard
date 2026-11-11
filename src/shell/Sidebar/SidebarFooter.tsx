import { Settings } from "lucide-react";
import { runBusCommand } from "../../state/requests";

/** Settings, right after the window's sidebar toggle. */
export function SidebarFooter() {
  return (
    <div className="sb-footer">
      <button type="button" className="sb-settings" aria-label="Settings" title="Settings (⌘,)" onClick={() => runBusCommand("app:settings", "Could not open Settings")}>
        <Settings size={16} strokeWidth={1.8} aria-hidden="true" />
      </button>
    </div>
  );
}
