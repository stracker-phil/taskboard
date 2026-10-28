import { useEffect, useState, type MouseEvent } from "react";
import { Kanban, List, Table2, type LucideIcon } from "lucide-react";
import { api, type Layout, type MenuEntry, type View } from "../api";
import { onSelectView } from "../commands/viewKeys";
import { InlineRename } from "../components/InlineRename";
import { useNavGo, useViews } from "../queries/sidebar";
import { attempt } from "../state/requests";
import { useStore } from "../state/store";

const LAYOUTS: Record<Layout, { name: string; icon: LucideIcon }> = {
  list: { name: "List", icon: List },
  board: { name: "Board", icon: Kanban },
  table: { name: "Table", icon: Table2 },
};
const LAYOUT_ORDER: Layout[] = ["list", "board", "table"];

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

  const addMenu = () =>
    void api.popupMenu(LAYOUT_ORDER.map((layout) => ({ text: LAYOUTS[layout].name, run: () => void create(layout) })));

  return (
    <div className="view-tabs" role="tablist" aria-label="Views">
      {views.map((view) => {
        const { icon: Icon } = LAYOUTS[view.layout];
        const tile = (
          <span className="view-tab__tile" data-layout={view.layout} aria-hidden="true">
            <Icon size={11} strokeWidth={2.8} />
          </span>
        );
        if (view.id === renamingId) {
          return (
            <div key={view.id} className="view-tab" data-active={view.id === viewId || undefined}>
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
            onClick={() => void open(listId, view.id)}
            onContextMenu={(e) => openMenu(e, view)}
          >
            {tile}
            {view.name}
          </button>
        );
      })}
      <span className="view-tabs__separator" aria-hidden="true" />
      <button type="button" className="view-tabs__add" onClick={addMenu}>
        + View
      </button>
    </div>
  );
}
