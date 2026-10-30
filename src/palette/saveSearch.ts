import { api, type Nav, type SidebarTree } from "../api";
import { attempt } from "../state/requests";

const NEW_LIST_PREFIX = "Search: ";

/** The current Space, else Everything: where a saved search goes. */
function saveTarget(tree: SidebarTree | undefined, nav: Nav | undefined): number | undefined {
  const listId = nav?.location?.listId;
  const space = tree?.spaces.find((s) => s.listId === listId || s.lists.some((l) => l.id === listId));
  return space?.listId ?? tree?.everything.id;
}

/** "Save as a filter list": a new list with `text contains` under the current Space or Everything, opened. */
export async function saveSearch(text: string, tree: SidebarTree | undefined, nav: Nav | undefined, go: (listId: number) => Promise<void>) {
  const parent = saveTarget(tree, nav);
  if (parent === undefined) return;
  const filter = { conditions: [{ field: "text", op: "contains" as const, value: text }] };
  const id = await attempt(api.listCreate(parent, `${NEW_LIST_PREFIX}${text}`, filter), "Could not save the search");
  if (id !== null) await go(id);
}
