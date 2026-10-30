import { type ReactNode, useEffect } from "react";
import { IssueView } from "../issue/IssueView";
import { useNav, useSidebar, useViews } from "../queries/sidebar";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { HoverPeek } from "../issue/hoverPeek/HoverPeek";
import { CurrentPeek } from "./CurrentPeek";
import { Header } from "./Header";
import { TokenBanner } from "./TokenBanner";
import { findLevel } from "./levels";
import { Sidebar } from "./Sidebar/Sidebar";
import { SidebarToggle } from "./SidebarToggle";
import { Toolbar } from "./toolbar/Toolbar";
import { ViewTabs } from "./ViewTabs";

interface Props {
  sidebarHidden: boolean;
  /** The view body under the header and the tabs. */
  children: ReactNode;
}

/** The main window's frame: the sidebar beside the inset content panel, and the sidebar toggle fixed in the
 *  lower-left corner over both. */
export function Window({ sidebarHidden, children }: Props) {
  const issueView = useStore((s) => s.issueView);
  useCloseIssueOnNavigation();
  useDiscardDraftsOnLeave();
  return (
    <div className={sidebarHidden ? "shell shell--sidebar-hidden" : "shell"}>
      {!sidebarHidden && <Sidebar />}
      <SidebarToggle sidebarHidden={sidebarHidden} />
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
              {/* The preview lies over the panel's full height, so opening it never narrows or reflows the view. */}
              <CurrentPeek />
            </>
          )}
        </main>
      </div>
      <HoverPeek />
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

/** Leaving a view drops its drafts and its text filter (ADR 0038); the Issue view, the preview and the Peek
 *  over it don't change the nav place, so they keep them. */
function useDiscardDraftsOnLeave() {
  const { data: nav } = useNav();
  const keepOnly = useStore((s) => s.keepOnlyNarrowing);
  const listId = nav?.location?.listId;
  const viewId = nav?.location?.viewId;
  useEffect(() => {
    if (listId === undefined || viewId === undefined) return;
    keepOnly(narrowingKey(listId, viewId));
  }, [listId, viewId, keepOnly]);
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
  // The Pinned page is one fixed list: no tabs, no toolbar.
  const pinned = level.kind === "home" && level.node.kind === "pinned";
  return (
    <>
      <Header level={level} />
      <TokenBanner />
      {!pinned && <ViewTabs listId={location.listId} viewId={location.viewId} />}
      {!pinned && view && <Toolbar key={location.listId} level={level} view={view} />}
    </>
  );
}
