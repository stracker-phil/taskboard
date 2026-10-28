import { useState } from "react";
import { Search } from "lucide-react";
import { api, type SpaceNode } from "../../api";
import { SpacePopover } from "../../components/SpaceEditor";
import { useNav, useNavGo, useSetExpanded, useSidebar } from "../../queries/sidebar";
import { usePageAction } from "../../commands/pageActions";
import { attempt } from "../../state/requests";
import { useStore } from "../../state/store";
import { NEW_LIST_NAME } from "./FilterListTree";
import { findLevel, spaceOf } from "../levels";
import { HomeRows } from "./HomeRows";
import { SidebarFooter } from "./SidebarFooter";
import type { SpaceEditFocus } from "./SpaceRow";
import { SpacesSection } from "./SpacesSection";

/** The main window's sidebar: the traffic-light strip, Search, Home, the Spaces tree and the footer. */
export function Sidebar() {
  const { data: tree } = useSidebar();
  const { data: nav } = useNav();
  const open = useNavGo();
  const setExpanded = useSetExpanded();
  const setPaletteOpen = useStore((s) => s.setPaletteOpen);
  const [renamingId, setRenamingId] = useState<number | null>(null);
  // The Space popover ("Colour and icon…", "Client label…") opens from here.
  const [editing, setEditing] = useState<{ space: SpaceNode; focus: SpaceEditFocus } | null>(null);

  const currentId = nav?.location?.listId ?? null;
  const activeSpace = tree && currentId !== null ? spaceOf(findLevel(tree, currentId)) : null;

  const homeOf = (kind: string) => tree?.home.find((node) => node.kind === kind);
  const goTo = (listId: number | undefined) => (listId === undefined ? null : () => void open(listId));
  usePageAction("go.everything", goTo(tree?.everything.id));
  usePageAction("go.recent", goTo(homeOf("recent")?.id));
  usePageAction("go.pinned", goTo(homeOf("pinned")?.id));
  // In the current Space (opened and named in place) or, outside any, under Everything.
  const newList = async () => {
    const parent = activeSpace?.listId ?? tree?.everything.id;
    if (parent === undefined) return;
    const id = await attempt(api.listCreate(parent, NEW_LIST_NAME, null), "Could not create the filter list");
    if (id === null) return;
    if (activeSpace && nav && !nav.expanded.includes(activeSpace.listId)) void setExpanded([...nav.expanded, activeSpace.listId]);
    await open(id);
    if (activeSpace) setRenamingId(id);
  };
  usePageAction("list.new", tree ? () => void newList() : null);

  return (
    <aside className="sidebar" aria-label="Sidebar">
      <div className="sidebar__traffic-lights" data-tauri-drag-region />
      <button type="button" className="sb-search" title="Search (⌘K)" onClick={() => setPaletteOpen(true)}>
        <Search size={15} strokeWidth={2} aria-hidden="true" />
        <span className="sb-search__label">Search</span>
        <kbd className="sb-kbd">⌘K</kbd>
      </button>
      {tree && <HomeRows rows={[tree.everything, ...tree.home, ...tree.pinnedLists]} activeId={currentId} open={(id) => void open(id)} />}
      {tree && nav && (
        <SpacesSection
          spaces={tree.spaces}
          nav={nav}
          activeSpace={activeSpace}
          open={open}
          setExpanded={(ids) => void setExpanded(ids)}
          renamingId={renamingId}
          setRenamingId={setRenamingId}
          onEdit={(space, focus) => setEditing({ space, focus })}
        />
      )}
      <SidebarFooter />
      {editing && <SpacePopover space={editing.space} focus={editing.focus} onClose={() => setEditing(null)} />}
    </aside>
  );
}
