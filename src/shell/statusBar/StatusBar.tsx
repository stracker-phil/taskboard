import { JiraActivityStatus } from "./JiraActivityStatus";
import { SyncStatus } from "./SyncStatus";

/** The strip under the content panel for what concerns the whole app (ADR 0060). The window's sidebar toggle
 *  sits at its left end while the sidebar is hidden. */
export function StatusBar() {
  return (
    <footer className="status-bar" data-tauri-drag-region>
      <JiraActivityStatus />
      <SyncStatus />
    </footer>
  );
}
