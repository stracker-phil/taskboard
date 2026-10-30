import { useEffect, useRef, useState, type MouseEvent } from "react";
import { api, type Layout, type View } from "../api";
import { usePageAction } from "../commands/pageActions";
import { useGlide } from "../lib/motion";
import { usePaletteMenu } from "../palette/menus";
import { usePinDefault } from "../pins/PinPopover";
import { pinOf, usePins } from "../queries/pins";
import { onSelectView } from "../commands/viewKeys";
import { LookPopover } from "../components/look/LookPopover";
import { openContextMenu, openMenu, type MenuEntry } from "../components/menu";
import { useDragReorder } from "./Sidebar/useDragReorder";
import { useNavGo, useViews } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";
import { LAYOUT_ORDER, LAYOUTS, LayoutTile } from "./viewLayouts";
import { ViewTab } from "./ViewTab";

/** The level's view tabs (layout tile and name), "+ View", and ⌘1–⌘9 for the nth tab. */
export function ViewTabs({ listId, viewId }: { listId: number; viewId: number }) {
  const { data: views = [] } = useViews(listId);
  const open = useNavGo();
  const [renamingId, setRenamingId] = useState<number | null>(null);
  const [lookingId, setLookingId] = useState<number | null>(null);
  const lookAnchor = useRef<HTMLElement | null>(null);
  const looking = views.find((view) => view.id === lookingId);
  const editLook = (view: View, tile: HTMLElement | null) => {
    lookAnchor.current = tile;
    setLookingId(view.id);
  };
  const addRef = useRef<HTMLButtonElement>(null);
  const { data: pins } = usePins();
  const pinDefault = usePinDefault();

  useEffect(
    () =>
      onSelectView((index) => {
        const view = views[index];
        if (view) void open(listId, view.id);
      }),
    [views, listId, open],
  );

  const create = async (layout: Layout) => {
    const view = await attempt(api.viewCreate(listId, LAYOUTS[layout].name, layout), "Could not add the view");
    if (view) await open(listId, view.id);
  };

  const remove = async (view: View) => {
    const confirmed = await useStore.getState().confirm({
      title: `Delete the view “${view.name}”?`,
      text: "Its layout, grouping, sort and columns go. The issues stay.",
      action: "Delete View",
    });
    if (!confirmed) return;
    const left = await attempt(api.viewDelete(view.id), "Could not delete the view");
    if (left && view.id === viewId) await open(listId);
  };

  // The layouts the level's Space offers (all of them outside the Spaces); every view of a level carries them.
  const offered = views[0]?.layouts ?? LAYOUT_ORDER;
  const offers = (layout: Layout) => offered.includes(layout);

  const setLayoutOf = (view: View, layout: Layout) => {
    if (view.layout !== layout) void attempt(api.viewSetLayout(view.id, layout), "Could not change the layout");
  };
  const bookmarking = (view: View): MenuEntry => {
    const pin = pinOf(pins?.active, "view", String(view.id));
    return pin
      ? { text: "Remove Bookmark", run: () => void attempt(api.pinRemove(pin.id), "Could not remove the bookmark") }
      : { text: "Bookmark View", run: () => void attempt(api.pinCreate("view", String(view.id), pinDefault, null), "Could not bookmark the view") };
  };
  const editing = (view: View): MenuEntry[] => [
    { text: "Rename", run: () => setRenamingId(view.id) },
    { text: "Icon and Color…", run: () => editLook(view, document.querySelector(`[data-view-look="${view.id}"]`)) },
    { text: "Duplicate", run: () => void attempt(api.viewDuplicate(view.id), "Could not duplicate the view") },
    bookmarking(view),
  ];
  const deleting = (view: View): MenuEntry => ({ text: "Delete", danger: true, enabled: views.length > 1, run: () => void remove(view) });
  const tabMenu = (e: MouseEvent, view: View): void =>
    openContextMenu(
      e,
      [
        ...editing(view),
        { caption: "Layout" },
        ...offered.map(
          (layout): MenuEntry => ({
            text: LAYOUTS[layout].name,
            icon: <LayoutTile layout={layout} />,
            checked: layout === view.layout,
            run: () => setLayoutOf(view, layout),
          }),
        ),
        "separator",
        deleting(view),
      ],
      view.name,
    );

  const addMenu = () => {
    if (!addRef.current) return;
    const layouts = offered.map(
      (layout): MenuEntry => ({ text: LAYOUTS[layout].name, icon: <LayoutTile layout={layout} />, hint: LAYOUTS[layout].hint, run: () => void create(layout) }),
    );
    openMenu([{ caption: "New view" }, ...layouts], addRef.current, "New view");
  };
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    const moved = views[from];
    if (moved) void attempt(api.viewMove(moved.id, to), "Could not move the view");
  }, "x");
  // A tab that is renamed, added or removed moves the ones after it smoothly.
  useGlide(containerRef);

  const current = views.findIndex((view) => view.id === viewId);
  const step = (by: 1 | -1) => {
    const next = views[(current + by + views.length) % views.length];
    if (next) void open(listId, next.id);
  };
  const setLayout = (layout: Layout) => {
    const view = views[current];
    if (view) setLayoutOf(view, layout);
  };
  usePageAction("view.new", addMenu);
  usePageAction("view.next", views.length > 1 ? () => step(1) : null);
  usePageAction("view.previous", views.length > 1 ? () => step(-1) : null);
  usePageAction("view.layout.list", current >= 0 && offers("list") ? () => setLayout("list") : null);
  usePageAction("view.layout.board", current >= 0 && offers("board") ? () => setLayout("board") : null);
  usePageAction("view.layout.table", current >= 0 && offers("table") ? () => setLayout("table") : null);
  const shown = views[current];
  usePaletteMenu("views.open", {
    section: "Views",
    title: "Open view",
    entries: () => views.map((view): MenuEntry => ({ text: view.name, checked: view.id === viewId, run: () => void open(listId, view.id) })),
  });
  usePaletteMenu("views.edit", shown ? { section: "Views", title: "View", entries: () => [...editing(shown), deleting(shown)] } : null);

  return (
    <div className="view-tabs" role="tablist" aria-label="Views" ref={containerRef}>
      {views.map((view, index) => (
        <ViewTab
          key={view.id}
          view={view}
          active={view.id === viewId}
          renaming={view.id === renamingId}
          onOpen={() => void open(listId, view.id)}
          onRename={() => setRenamingId(view.id)}
          onRenameDone={() => setRenamingId(null)}
          onEditLook={(tile) => (lookingId === view.id ? setLookingId(null) : editLook(view, tile))}
          onMenu={(e) => tabMenu(e, view)}
          dragProps={view.id === renamingId ? item(index) : { ...item(index), ...handle(index) }}
        />
      ))}
      {dropLineY !== null && <span className="view-tabs__drop" style={{ left: dropLineY }} aria-hidden="true" />}
      <span className="view-tabs__separator" aria-hidden="true" />
      <button ref={addRef} type="button" className="view-tabs__add" aria-haspopup="menu" onClick={addMenu}>
        + View
      </button>
      {looking && (
        <LookPopover
          title="Icon and color"
          look={{ icon: looking.icon, color: looking.color }}
          fallback={LAYOUTS[looking.layout].icon}
          colorOptional
          anchor={lookAnchor}
          onChange={(look) => void attempt(api.viewSetLook(looking.id, look.icon, look.color), "Could not change the view's icon")}
          onClose={() => setLookingId(null)}
        />
      )}
    </div>
  );
}
