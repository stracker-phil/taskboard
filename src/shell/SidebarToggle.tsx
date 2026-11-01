import { PanelLeft } from "lucide-react";
import { useShortcuts } from "../commands/useShortcuts";
import { runBusCommand } from "../state/requests";

/** Shows or hides the sidebar from the window's lower-left corner, the same spot in both states. */
export function SidebarToggle({ sidebarHidden }: { sidebarHidden: boolean }) {
  const label = sidebarHidden ? "Show sidebar" : "Hide sidebar";
  const { title } = useShortcuts();
  return (
    <button
      type="button"
      className="sidebar-toggle"
      aria-label={label}
      aria-pressed={!sidebarHidden}
      title={title(label, "sidebar.toggle")}
      onClick={() => runBusCommand("view:toggleSidebar", "Could not toggle the sidebar")}
    >
      <PanelLeft size={17} strokeWidth={1.8} aria-hidden="true" />
    </button>
  );
}
