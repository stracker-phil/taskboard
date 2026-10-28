import { useEffect, useRef, useState, type MouseEvent } from "react";
import { Kanban, List, Table2, type LucideIcon } from "lucide-react";
import { api, type Layout, type MenuEntry, type View } from "../api";
import { usePageAction } from "../commands/pageActions";
import { onSelectView } from "../commands/viewKeys";
import { InlineRename } from "../components/InlineRename";
import { Popover } from "../components/Popover";
import { useDragReorder } from "./Sidebar/useDragReorder";
import { useNavGo, useViews } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

const LAYOUTS: Record<Layout, { name: string; icon: LucideIcon }> = {
  list: { name: "List", icon: List },
  board: { name: "Board", icon: Kanban },
  table: { name: "Table", icon: Table2 },
};
const LAYOUT_ORDER: Layout[] = ["list", "board", "table"];
const LAYOUT_HINTS: Record<Layout, string> = { list: "Groups of rows", board: "Status columns", table: "A dense grid" };

/** The level's view tabs (layout tile and name), "+ View", and ⌘1–⌘9 for the nth tab. */
export function ViewTabs({ listId, viewId }: { listId: number; viewId: number }) {
  const { data: views = [] } = useViews(listId);
  const open = useNavGo();
  const [renamingId, setRenamingId] = useState<number | null>(null);

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

  const openMenu = (e: MouseEvent, view: View) => {
    e.preventDefault();
    const layouts: MenuEntry[] = LAYOUT_ORDER.map((layout) => ({
      text: LAYOUTS[layout].name,
      enabled: layout !== view.layout,
      run: () => void attempt(api.viewSetLayout(view.id, layout), "Could not change the layout"),
    }));
    void api.popupMenu([
      { text: "Rename", run: () => setRenamingId(view.id) },
      { text: "Duplicate", run: () => void attempt(api.viewDuplicate(view.id), "Could not duplicate the view") },
      { text: "Change layout", items: layouts },
      "separator",
      { text: "Delete", enabled: views.length > 1, run: () => void remove(view) },
    ]);
  };

  const [adding, setAdding] = useState(false);
  const addRef = useRef<HTMLButtonElement>(null);
  const addMenu = () => setAdding(true);
  const { containerRef, item, handle, dropLineY } = useDragReorder<HTMLDivElement>((from, to) => {
    const moved = views[from];
    if (moved) void attempt(api.viewMove(moved.id, to), "Could not move the view");
  }, "x");

  const current = views.findIndex((view) => view.id === viewId);
  const step = (by: 1 | -1) => {
    const next = views[(current + by + views.length) % views.length];
    if (next) void open(listId, next.id);
  };
  const setLayout = (layout: Layout) => {
    const view = views[current];
    if (view && view.layout !== layout) void attempt(api.viewSetLayout(view.id, layout), "Could not change the layout");
  };
  usePageAction("view.new", addMenu);
  usePageAction("view.next", views.length > 1 ? () => step(1) : null);
  usePageAction("view.previous", views.length > 1 ? () => step(-1) : null);
  usePageAction("view.layout.list", current >= 0 ? () => setLayout("list") : null);
  usePageAction("view.layout.board", current >= 0 ? () => setLayout("board") : null);
  usePageAction("view.layout.table", current >= 0 ? () => setLayout("table") : null);

  return (
    <div className="view-tabs" role="tablist" aria-label="Views" ref={containerRef}>
      {views.map((view, index) => {
        const { icon: Icon } = LAYOUTS[view.layout];
        const tile = (
          <span className="view-tab__tile" data-layout={view.layout} aria-hidden="true">
            <Icon size={11} strokeWidth={2.8} />
          </span>
        );
        if (view.id === renamingId) {
          return (
            <div key={view.id} className="view-tab" data-active={view.id === viewId || undefined} {...item(index)}>
              {tile}
              <InlineRename
                value={view.name}
                label="View name"
                onSave={(name) => void attempt(api.viewRename(view.id, name), "Could not rename the view")}
                onDone={() => setRenamingId(null)}
              />
            </div>
          );
        }
        return (
          <button
            key={view.id}
            type="button"
            role="tab"
            className="view-tab"
            aria-selected={view.id === viewId}
            data-active={view.id === viewId || undefined}
            title="Double-click to rename, drag to move"
            onClick={() => void open(listId, view.id)}
            onDoubleClick={() => setRenamingId(view.id)}
            onContextMenu={(e) => openMenu(e, view)}
            {...item(index)}
            {...handle(index)}
          >
            {tile}
            {view.name}
          </button>
        );
      })}
      {dropLineY !== null && <span className="view-tabs__drop" style={{ left: dropLineY }} aria-hidden="true" />}
      <span className="view-tabs__separator" aria-hidden="true" />
      <span className="view-tabs__anchor">
        <button ref={addRef} type="button" className="view-tabs__add" aria-expanded={adding} onClick={() => setAdding((was) => !was)}>
          + View
        </button>
        {adding && (
          <Popover label="New view" anchor={addRef} onClose={() => setAdding(false)} className="view-add">
            <span className="popover__caps">New view</span>
            {LAYOUT_ORDER.map((layout) => {
              const { icon: Icon, name } = LAYOUTS[layout];
              return (
                <button
                  key={layout}
                  type="button"
                  className="view-add__option"
                  onClick={() => {
                    setAdding(false);
                    void create(layout);
                  }}
                >
                  <span className="view-tab__tile" data-layout={layout} aria-hidden="true">
                    <Icon size={11} strokeWidth={2.8} />
                  </span>
                  {name}
                  <span className="view-add__hint">{LAYOUT_HINTS[layout]}</span>
                </button>
              );
            })}
          </Popover>
        )}
      </span>
    </div>
  );
}
