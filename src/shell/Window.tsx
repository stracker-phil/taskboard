import { type ReactNode, useEffect } from "react";
import { IssueView } from "../issue/IssueView";
import { useNav, useSidebar, useViews } from "../queries/sidebar";
import { useStore } from "../state/store";
import { CurrentPeek } from "./CurrentPeek";
import { Header } from "./Header";
import { findLevel } from "./levels";
import { Sidebar } from "./Sidebar/Sidebar";
import { Toolbar } from "./toolbar/Toolbar";
import { ViewTabs } from "./ViewTabs";

interface Props {
  sidebarHidden: boolean;
  /** The view body under the header and the tabs. */
  children: ReactNode;
}

/** The main window's frame: the sidebar beside the inset content panel. */
export function Window({ sidebarHidden, children }: Props) {
  const issueView = useStore((s) => s.issueView);
  useCloseIssueOnNavigation();
  return (
    <div className={sidebarHidden ? "shell shell--sidebar-hidden" : "shell"}>
      {!sidebarHidden && <Sidebar />}
      <div className="shell__content">
        {sidebarHidden && <div className="shell__titlebar" data-tauri-drag-region />}
        <main className="content-panel">
          {issueView ? (
            <IssueView key={issueView.stack.length} id={issueView.stack[issueView.stack.length - 1]} origin={issueView.origin} />
          ) : (
            <>
              <div className="content-panel__main">
                <LevelChrome />
                <div className="content-panel__body">{children}</div>
              </div>
              <CurrentPeek />
            </>
          )}
        </main>
      </div>
    </div>
  );
}

/** Going to another level or view (sidebar, tabs, keys) leaves the Issue view. */
function useCloseIssueOnNavigation() {
  const { data: nav } = useNav();
  const closeIssue = useStore((s) => s.closeIssue);
  const listId = nav?.location?.listId;
  const viewId = nav?.location?.viewId;
  useEffect(() => {
    const open = useStore.getState().issueView;
    // Before the place has loaded nothing has moved yet.
    if (listId === undefined) return;
    if (open && (open.origin.listId !== listId || open.origin.viewId !== viewId)) closeIssue();
  }, [listId, viewId, closeIssue]);
}

/** The current level's header, view tabs and toolbar, once the tree and the place are known. */
function LevelChrome() {
  const { data: tree } = useSidebar();
  const { data: nav } = useNav();
  const location = nav?.location;
  const { data: views } = useViews(location?.listId ?? null);
  const level = tree && location ? findLevel(tree, location.listId) : null;
  if (!level || !location) return null;
  const view = views?.find((v) => v.id === location.viewId);
  return (
    <>
      <Header level={level} />
      <ViewTabs listId={location.listId} viewId={location.viewId} />
      {view && <Toolbar level={level} view={view} />}
    </>
  );
}
