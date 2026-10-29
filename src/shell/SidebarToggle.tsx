import { PanelLeft } from "lucide-react";
import { bindingFor, bindingLabel } from "../commands/registry";
import { runBusCommand } from "../state/requests";

/** Shows or hides the sidebar from the window's lower-left corner, the same spot in both states. */
export function SidebarToggle({ sidebarHidden }: { sidebarHidden: boolean }) {
  const label = sidebarHidden ? "Show sidebar" : "Hide sidebar";
  const shortcut = bindingLabel(bindingFor("sidebar.toggle"));
  return (
    <button
      type="button"
      className="sidebar-toggle"
      aria-label={label}
      aria-pressed={!sidebarHidden}
      title={shortcut ? `${label} (${shortcut})` : label}
      onClick={() => runBusCommand("view:toggleSidebar", "Could not toggle the sidebar")}
    >
      <PanelLeft size={17} strokeWidth={1.8} aria-hidden="true" />
    </button>
  );
}
