import { X } from "lucide-react";
import { type MouseEvent, useEffect } from "react";
import type { Nav, Tab } from "../api";
import { onSelectTab } from "../commands/tabKeys";
import { useShortcuts } from "../commands/useShortcuts";
import { openContextMenu } from "../components/menu";
import { useIssue } from "../queries/issues";
import { useSidebar, useTabs, useViews } from "../queries/sidebar";
import { useStore } from "../state/store";
import { findLevel } from "./levels";
import { useDragReorder } from "./Sidebar/useDragReorder";

/** The main window's tabs (ADR 0050), shown while more than one is open: each its level and view, or the issue it
 *  shows; a click or ⌘⇧1–⌘⇧9 shows one, × or a middle click closes it, a drag moves it. Tabs share the strip's width
 *  down to a floor; past it the strip scrolls sideways. */
export function TabStrip({ nav }: { nav: Nav }) {
  const { activate, close, move, open } = useTabs();
  const { key } = useShortcuts();
  const tabs = nav.tabs;
  useEffect(
    () =>
      onSelectTab((index) => {
        const tab = tabs[index];
        if (tab && tab.id !== nav.activeTab) void activate(tab.id);
      }),
    [tabs, nav.activeTab, activate],
  );
  // `to` counts the tabs without the dragged one, as the bookmarks bar does.
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    const tab = tabs[from];
    if (!tab) return;
    const rest = tabs.filter((other) => other.id !== tab.id);
    void move(tab.id, rest[to]?.id ?? null);
  }, "x");

  const menu = (e: MouseEvent, tab: Tab) =>
    openContextMenu(
      e,
      [
        { text: "New Tab", enabled: tab.location !== null, run: () => tab.location && void open(tab.location.listId, tab.location.viewId, true) },
        "separator",
        { text: "Close Tab", run: () => void close(tab.id) },
        { text: "Close Other Tabs", enabled: tabs.length > 1, run: () => void closeOthers(tab.id) },
      ],
      "Tab",
    );
  const closeOthers = async (keep: number) => {
    for (const other of tabs) if (other.id !== keep) await close(other.id);
    await activate(keep);
  };

  return (
    <nav className="tab-strip" aria-label="Tabs" data-tauri-drag-region>
      <div ref={containerRef} className="tab-strip__tabs" role="tablist" data-tauri-drag-region>
        {tabs.map((tab, index) => {
          const active = tab.id === nav.activeTab;
          return (
            <div
              key={tab.id}
              role="tab"
              aria-selected={active}
              tabIndex={active ? 0 : -1}
              className={active ? "tab-strip__tab tab-strip__tab--active" : "tab-strip__tab"}
              {...item(index)}
              {...handle(index)}
              onClick={() => !active && void activate(tab.id)}
              onAuxClick={(e) => e.button === 1 && void close(tab.id)}
              onContextMenu={(e) => menu(e, tab)}
            >
              <TabLabel tab={tab} active={active} shortcut={index < 9 ? key(`tab.select.${index + 1}`) : ""} />
              <button
                type="button"
                className="tab-strip__close"
                aria-label="Close tab"
                title="Close tab (⌘W)"
                onPointerDown={(e) => e.stopPropagation()}
                onClick={(e) => {
                  e.stopPropagation();
                  void close(tab.id);
                }}
              >
                <X size={12} strokeWidth={2.2} aria-hidden="true" />
              </button>
            </div>
          );
        })}
        {dropLineY !== null && <span className="tab-strip__drop" style={{ left: dropLineY }} aria-hidden="true" />}
      </div>
    </nav>
  );
}

/** "Level · View", the way the window's header names the place, or the key and title of the issue the tab shows. */
function TabLabel({ tab, active, shortcut }: { tab: Tab; active: boolean; shortcut: string }) {
  const { data: tree } = useSidebar();
  const { data: views } = useViews(tab.location?.listId ?? null);
  const issueView = useStore((s) => (active ? s.issueView : s.parkedIssueViews[tab.id]));
  const { data: issue } = useIssue(issueView?.stack.at(-1) ?? null);
  const level = tree && tab.location ? findLevel(tree, tab.location.listId) : null;
  const levelName = !level ? "" : level.kind === "space" ? level.space.name : level.node.name;
  const view = views?.find((v) => v.id === tab.location?.viewId);
  const place = view && views && views.length > 1 ? `${levelName} · ${view.name}` : levelName;
  const name = issueView && issue ? `${issue.key} ${issue.summary}` : place;
  return (
    <span className="tab-strip__name" title={shortcut ? `${name} (${shortcut})` : name}>
      {name}
    </span>
  );
}
