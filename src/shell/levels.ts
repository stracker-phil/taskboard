import { Bookmark, Funnel, History, Layers2, type LucideIcon } from "lucide-react";
import type { ListKind, ListNode, SidebarTree, SpaceNode } from "../api";

/** A sidebar level as the page draws it: Everything or a Home list, a Space, or a filter list in a Space. */
export type Level =
  | { kind: "home"; node: ListNode }
  | { kind: "space"; space: SpaceNode }
  | { kind: "list"; space: SpaceNode; node: ListNode };

/** The level with this id in the tree, if it is still there. */
export function findLevel(tree: SidebarTree, listId: number): Level | null {
  const home = [tree.everything, ...tree.home].find((node) => node.id === listId);
  if (home) return { kind: "home", node: home };
  for (const space of tree.spaces) {
    if (space.listId === listId) return { kind: "space", space };
    const node = space.lists.find((list) => list.id === listId);
    if (node) return { kind: "list", space, node };
  }
  return null;
}

/** The Space a level belongs to, if any. */
export const spaceOf = (level: Level | null): SpaceNode | null => (level && level.kind !== "home" ? level.space : null);

/** The icon of a Home level: Everything, Unread, Bookmarks, or a funnel for a filter list. */
export const HOME_ICONS: Record<ListKind, LucideIcon> = {
  everything: Layers2,
  recent: History,
  pinned: Bookmark,
  list: Funnel,
  space: Funnel,
};
