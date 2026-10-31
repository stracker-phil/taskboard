import type { Group } from "../../api";
import type { CardPosition, Item } from "./flatten";

type RowItem = Extract<Item, { kind: "row" }>;

/** A group as the List shows it while an issue is dragged. */
export interface DragGroup {
  group: Group;
  /** Draws the group's tab (a grouped view). */
  tab: boolean;
  first: boolean;
  /** At most `shown` rows, the dragged one always among them, with their card positions. */
  rows: RowItem[];
  /** The group's rows that don't show. */
  more: number;
}

/** Every group of the view, closed ones too, each with the same few rows, so every drop zone is the same size and
 *  the dragged row stays in sight in its own group. `items`: the view flattened with every group open. */
export function dragGroups(items: Item[], draggedId: string, shown: number): DragGroup[] {
  const groups: (DragGroup & { total: number })[] = [];
  for (const item of items) {
    if (item.kind === "group") {
      groups.push({ group: item.group, tab: item.tab, first: item.first, rows: [], more: 0, total: 0 });
      continue;
    }
    const current = groups[groups.length - 1];
    if (!current) continue;
    current.total++;
    if (current.rows.length < shown) current.rows.push(item);
    else if (item.row.id === draggedId) current.rows[shown - 1] = item;
  }
  return groups.map(({ total, ...group }) => ({
    ...group,
    rows: group.rows.map((row, i) => ({ ...row, position: positionOf(i, group.rows.length) })),
    more: total - group.rows.length,
  }));
}

function positionOf(index: number, count: number): CardPosition {
  if (count === 1) return "only";
  if (index === 0) return "first";
  return index === count - 1 ? "last" : "middle";
}
