import type { ReactNode } from "react";

interface Props {
  sidebarHidden: boolean;
  children: ReactNode;
}

/** The main window's frame: the sidebar beside the inset content panel. Later steps fill the regions. */
export function Window({ sidebarHidden, children }: Props) {
  return (
    <div className={sidebarHidden ? "shell shell--sidebar-hidden" : "shell"}>
      {!sidebarHidden && <Sidebar />}
      <div className="shell__content">
        {sidebarHidden && <div className="shell__titlebar" data-tauri-drag-region />}
        <main className="content-panel">{children}</main>
      </div>
    </div>
  );
}

function Sidebar() {
  return (
    <aside className="sidebar" aria-label="Sidebar">
      <div className="sidebar__traffic-lights" data-tauri-drag-region />
      <nav className="sidebar__home" aria-label="Home" />
      <nav className="sidebar__spaces" aria-label="Spaces" />
      <div className="sidebar__footer" />
    </aside>
  );
}
