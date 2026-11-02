import { Kanban, List, Table2, type LucideIcon } from "lucide-react";
import type { Layout } from "../api";
import { Tile, type Look } from "../components/look/Tile";

export const LAYOUTS: Record<Layout, { name: string; icon: LucideIcon; hint: string }> = {
  list: { name: "List", icon: List, hint: "Groups of rows" },
  board: { name: "Board", icon: Kanban, hint: "Status columns" },
  table: { name: "Table", icon: Table2, hint: "A dense grid" },
};
export const LAYOUT_ORDER: Layout[] = ["list", "board", "table"];

/** A layout's glyph on its color. */
export function LayoutTile({ layout }: { layout: Layout }) {
  const { icon: Icon } = LAYOUTS[layout];
  return (
    <span className="view-tab__tile" data-layout={layout} aria-hidden="true">
      <Icon size={11} strokeWidth={2.8} />
    </span>
  );
}

/** A view's icon: its own look, or its layout's tile when it has none. */
export function ViewTile({ layout, icon, color }: Look & { layout: Layout }) {
  if (!icon && !color) return <LayoutTile layout={layout} />;
  return <Tile icon={icon} color={color} fallback={LAYOUTS[layout].icon} size={color ? 18 : 15} />;
}
