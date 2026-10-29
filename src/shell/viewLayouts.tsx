import { Kanban, List, Table2, type LucideIcon } from "lucide-react";
import type { Layout } from "../api";

export const LAYOUTS: Record<Layout, { name: string; icon: LucideIcon; hint: string }> = {
  list: { name: "List", icon: List, hint: "Groups of rows" },
  board: { name: "Board", icon: Kanban, hint: "Status columns" },
  table: { name: "Table", icon: Table2, hint: "A dense grid" },
};
export const LAYOUT_ORDER: Layout[] = ["list", "board", "table"];

/** A layout's glyph on its color: the view tabs and the layout rows of their menus. */
export function LayoutTile({ layout }: { layout: Layout }) {
  const { icon: Icon } = LAYOUTS[layout];
  return (
    <span className="view-tab__tile" data-layout={layout} aria-hidden="true">
      <Icon size={11} strokeWidth={2.8} />
    </span>
  );
}
