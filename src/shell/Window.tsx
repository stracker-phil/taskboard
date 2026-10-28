import type { ReactNode } from "react";
import { useNav, useSidebar } from "../queries/sidebar";
import { Header } from "./Header";
import { findLevel } from "./levels";
import { Sidebar } from "./Sidebar/Sidebar";
import { ViewTabs } from "./ViewTabs";

interface Props {
  sidebarHidden: boolean;
  /** The view body under the header and the tabs. */
  children: ReactNode;
}

/** The main window's frame: the sidebar beside the inset content panel. */
export function Window({ sidebarHidden, children }: Props) {
  return (
    <div className={sidebarHidden ? "shell shell--sidebar-hidden" : "shell"}>
      {!sidebarHidden && <Sidebar />}
      <div className="shell__content">
        {sidebarHidden && <div className="shell__titlebar" data-tauri-drag-region />}
        <main className="content-panel">
          <LevelChrome />
          <div className="content-panel__body">{children}</div>
        </main>
      </div>
    </div>
  );
}

/** The current level's header and view tabs, once the tree and the place are known. */
function LevelChrome() {
  const { data: tree } = useSidebar();
  const { data: nav } = useNav();
  const location = nav?.location;
  const level = tree && location ? findLevel(tree, location.listId) : null;
  if (!level || !location) return null;
  return (
    <>
      <Header level={level} />
      <ViewTabs listId={location.listId} viewId={location.viewId} />
    </>
  );
}
