import { type ReactNode, useEffect, useLayoutEffect, useRef } from "react";
import { IssueView } from "../issue/IssueView";
import { BookmarksBar } from "../pins/BookmarksBar";
import { useModes } from "../queries/modes";
import { useNav, useReportShownIssue, useSidebar, useViews } from "../queries/sidebar";
import { narrowingKey } from "../state/slices/narrowing";
import { useStore } from "../state/store";
import { HoverPeek } from "../issue/hoverPeek/HoverPeek";
import { CurrentPeek } from "./CurrentPeek";
import { Header } from "./Header";
import { TokenBanner } from "./TokenBanner";
import { findLevel } from "./levels";
import { Sidebar } from "./Sidebar/Sidebar";
import { NavArrows } from "./NavArrows";
import { SidebarToggle } from "./SidebarToggle";
import { TabStrip } from "./TabStrip";
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
  const { bookmarksBar } = useModes();
  const { data: nav } = useNav();
  const tabbed = (nav?.tabs.length ?? 0) > 1;
  const tabStrip = tabbed && nav ? <TabStrip nav={nav} /> : null;
  const tabsInTitlebar = sidebarHidden && !bookmarksBar;
  const tabsRow = tabbed && !tabsInTitlebar;
  useIssueViewFollowsTheTab();
  useReportShownIssue(issueView ? issueView.stack[issueView.stack.length - 1] : null);
  useDiscardDraftsOnLeave();
  return (
    <div className={sidebarHidden ? "shell shell--sidebar-hidden" : "shell"}>
      {!sidebarHidden && <Sidebar />}
      <SidebarToggle sidebarHidden={sidebarHidden} />
      <NavArrows />
      {/* The gaps around the panel drag the window too. */}
      <div className="shell__content" data-tauri-drag-region>
        {sidebarHidden ? (
          // The strip that keeps the traffic lights off the panel holds the bookmarks bar, or else the tabs.
          <div className="shell__titlebar" data-tauri-drag-region>
            {bookmarksBar ? <BookmarksBar /> : tabStrip}
          </div>
        ) : (
          // Always there, so turning the bar on or off grows or shrinks it and the panel glides.
          <div className={bookmarksBar ? "shell__bookmarks shell__bookmarks--open" : "shell__bookmarks"} inert={!bookmarksBar}>
            {bookmarksBar && <BookmarksBar />}
          </div>
        )}
        {/* Always there, so a second tab or the bar pushing the tabs down grows it and the panel glides. */}
        <div className={tabsRow ? "shell__tabs shell__tabs--open" : "shell__tabs"} inert={!tabsRow}>
          {tabsRow && tabStrip}
        </div>
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

/** Going to another level or view (sidebar, view tabs, keys) leaves the Issue view. Switching tabs parks it with
 *  its tab and brings back the other tab's, before the frame is painted, so one tab's Issue view never shows over
 *  another's place. */
function useIssueViewFollowsTheTab() {
  const { data: nav } = useNav();
  const closeIssue = useStore((s) => s.closeIssue);
  const switchTab = useStore((s) => s.switchTab);
  const shownTab = useRef<number | null>(null);
  const listId = nav?.location?.listId;
  const viewId = nav?.location?.viewId;
  const activeTab = nav?.activeTab;
  const tabIds = nav?.tabs.map((tab) => tab.id).join(",") ?? "";
  useLayoutEffect(() => {
    // Before the place has loaded nothing has moved yet.
    if (listId === undefined || activeTab === undefined) return;
    const from = shownTab.current;
    shownTab.current = activeTab;
    if (from !== null && from !== activeTab) return switchTab(from, activeTab, tabIds.split(",").map(Number));
    const open = useStore.getState().issueView;
    if (open && (open.origin.listId !== listId || open.origin.viewId !== viewId)) closeIssue();
  }, [listId, viewId, activeTab, tabIds, closeIssue, switchTab]);
}

/** Leaving a view drops its drafts and its text filter (ADR 0038); the Issue view, the preview and the Peek
 *  over it don't change the nav place, so they keep them. Every open tab's view keeps its own. */
function useDiscardDraftsOnLeave() {
  const { data: nav } = useNav();
  const keep = useStore((s) => s.keepNarrowings);
  const keys = nav?.tabs.flatMap((tab) => (tab.location ? [narrowingKey(tab.location.listId, tab.location.viewId)] : [])).join("|");
  useEffect(() => {
    if (keys === undefined || keys === "") return;
    keep(keys.split("|"));
  }, [keys, keep]);
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
